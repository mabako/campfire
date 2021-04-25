mod commandline;
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
use log::{debug, error, info};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tera::{Context, Tera};

fn main() {
    if let Err(_) = std::env::var("LOG") {
        std::env::set_var("LOG", "info");
    }
    pretty_env_logger::init_custom_env("LOG");

    let matches = commandline::parse_command().get_matches();

    let base_dir = PathBuf::from(matches.value_of("base-directory").unwrap());
    if !base_dir.exists() {
        panic!(
            "Could not read base directory: {}",
            base_dir.to_str().unwrap()
        );
    }

    let config_path = matches.value_of("config").unwrap();
    let config_file = base_dir.join(config_path);
    let config = read_config(&config_file)
        .unwrap_or_else(|_| panic!("Could not read config: {}", config_file.to_str().unwrap()));
    info!(
        "Generating site {} using config {}",
        base_dir.to_str().unwrap(),
        config_file.to_str().unwrap()
    );

    match matches.subcommand() {
        ("build", _) => build(base_dir.into(), config.into()),
        _ => panic!(),
    }
    info!("Done.");
}

fn build(base_dir: PathBuf, config: Config) {
    let template_path = base_dir
        .join(".campfire")
        .join(config.template_path.clone())
        .canonicalize()
        .unwrap()
        .join("**")
        .join("*.html");
    let template_path = template_path.to_str().unwrap();
    debug!("Using templates from {}", template_path);
    let tera = match Tera::new(template_path) {
        Ok(t) => t,
        Err(e) => {
            error!("Tera: Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    let files = dir::find_all_markdown_files(&base_dir, &config);

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

    info!("Generating {}", file_dir.to_str().unwrap());

    let rendered = tera.render("post.html", &context).unwrap();
    fs::create_dir_all(&file_dir).expect("Failed to create directory");
    fs::write(output_file, rendered).expect("Failed to write output");

    for asset in assets {
        let asset_source_path = base_directory.join(&asset.source);
        let asset_target_path = output_dir.join(&asset.target);
        debug!("  Copying asset {}", asset_target_path.to_str().unwrap());
        fs::copy(asset_source_path, asset_target_path).unwrap();
    }

    return post_context;
}

fn run_post_build_command(post_build_command: String, campfire_dir: PathBuf) {
    if !post_build_command.is_empty() {
        info!("Running post-build command: {}", post_build_command);
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
