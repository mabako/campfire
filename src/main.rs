mod config;
mod context;
mod dir;
mod markdown;

#[macro_use]
extern crate lazy_static;
use crate::config::{read_config, Config};
use crate::context::PostContext;
use crate::markdown::MarkdownFile;
use chrono::Datelike;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};
use tera::{Context, Tera};

fn main() {
    if std::env::args().len() != 2 {
        panic!("No base directory provided")
    }
    let base_directory = &env::args().nth(1).unwrap();
    let base_dir = Path::new(base_directory);
    let config = read_config(base_dir).expect("Could not read config");
    println!("{:?}", config);

    let template_path = base_dir
        .join(".campfire")
        .join(config.template_path.clone())
        .canonicalize()
        .unwrap()
        .join("**")
        .join("*.html");
    println!("Using templates from {:?}", template_path);
    let tera = match Tera::new(template_path.to_str().unwrap()) {
        Ok(t) => t,
        Err(e) => {
            println!("Tera: Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    let files = dir::find_all_markdown_files(&base_dir, &config);
    println!("Generating {} files", files.len());

    let campfire_dir = base_dir.join(".campfire");
    let output_dir = campfire_dir.join(config.target_path.clone());
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir).unwrap();
    }
    fs::create_dir(&output_dir).unwrap();
    fs::create_dir(output_dir.join("static")).unwrap();

    // render individual posts
    let mut posts: Vec<PostContext> = Vec::new();
    for file in files {
        posts.push(generate_post_and_copy_assets(
            &config,
            &tera,
            &base_dir,
            &output_dir,
            file,
        ));
    }
    posts.sort_by(|a, b| b.date.cmp(&a.date));

    // render index
    let mut context = Context::new();
    context.insert("posts", &posts);
    // TODO pass a global context around and extend sub-contexts from it
    context.insert("base_url", &config.base_url);

    let rendered = tera.render("index.html", &context).unwrap();
    fs::write(output_dir.join("index.html"), rendered).expect("Failed to write output");

    run_post_build_command(config.post_build_command.into(), campfire_dir);

    println!("Done.");
}

fn generate_post_and_copy_assets(
    config: &Config,
    tera: &Tera,
    base_directory: &Path,
    output_dir: &Path,
    file: MarkdownFile,
) -> PostContext {
    let file_dir = output_dir.join(file.slug(base_directory));
    let output_file = file_dir.join("index.html");

    let (html, assets) = &file.render_to_html(&config);
    let post_context = PostContext {
        title: file.title(),
        date: file
            .frontmatter
            .date
            .unwrap()
            .format("%Y-%m-%d")
            .to_string(),
        year: file.frontmatter.date.unwrap().year(),
        month: file.frontmatter.date.unwrap().month(),
        day: file.frontmatter.date.unwrap().day(),
        markdown: html.into(),
        relative_url: format!("{}/", file.slug(base_directory)),
    };

    let mut context = Context::new();
    context.insert("post", &post_context);
    context.insert("base_url", &config.base_url);

    println!(
        "Writing {:?} -- {:?} {:?}",
        file_dir, file.path, file.frontmatter
    );

    let rendered = tera.render("post.html", &context).unwrap();
    fs::create_dir_all(&file_dir).expect("Failed to create directory");
    fs::write(output_file, rendered).expect("Failed to write output");

    for asset in assets {
        let asset_source_path = base_directory.join(&asset.source);
        let asset_target_path = output_dir.join(&asset.target);
        println!(
            "Copying asset {:?} from {:?}",
            asset_target_path, asset_source_path
        );
        fs::copy(asset_source_path, asset_target_path).unwrap();
    }

    return post_context;
}

fn run_post_build_command(post_build_command: String, campfire_dir: PathBuf) {
    if !post_build_command.is_empty() {
        println!("Running post-build command: {}", post_build_command);
        if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", &post_build_command])
                .current_dir(campfire_dir)
                .stdout(Stdio::inherit())
                .spawn()
                .expect("failed to execute post-build command");
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(post_build_command)
                .current_dir(campfire_dir)
                .stdout(Stdio::inherit())
                .spawn()
                .expect("failed to execute post-build command");
        }
    }
}
