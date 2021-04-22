mod config;
mod dir;
mod markdown;

#[macro_use]
extern crate lazy_static;
use crate::config::{read_config, Config};
use crate::markdown::MarkdownFile;
use std::path::Path;
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

    for file in files {
        generate_markdown_and_copy_assets(&config, &tera, &base_dir, &output_dir, file);
    }

    if !config.post_build_command.is_empty() {
        println!("Running post-build command: {}", config.post_build_command);
        if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", &config.post_build_command])
                .current_dir(campfire_dir)
                .stdout(Stdio::inherit())
                .spawn()
                .expect("failed to execute post-build command");
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(config.post_build_command)
                .current_dir(campfire_dir)
                .stdout(Stdio::inherit())
                .spawn()
                .expect("failed to execute post-build command");
        }
    }

    println!("Done.");
}

fn generate_markdown_and_copy_assets(
    config: &Config,
    tera: &Tera,
    base_directory: &Path,
    output_dir: &Path,
    file: MarkdownFile,
) {
    let mut context = Context::new();
    context.insert("title", &file.title());
    context.insert(
        "date",
        &file
            .frontmatter
            .date
            .unwrap()
            .format("%Y-%m-%d")
            .to_string(),
    );
    context.insert("base_url", &config.base_url);

    let (html, assets) = &file.render_to_html(&config);
    context.insert("markdown", &html);

    let file_dir = output_dir.join(file.slug(base_directory));
    let output_file = file_dir.join("index.html");
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
}
