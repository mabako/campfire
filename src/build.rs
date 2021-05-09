use crate::config::Config;
use crate::context::{GeneratorContext, PostContext};
use crate::dir;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tera::{Context, Tera};

use crate::markdown::MarkdownFile;
use chrono::Datelike;
use log::{debug, error, info, warn};

pub fn build(base_dir: PathBuf, config: Config) {
    let template_path = base_dir
        .join(".campfire")
        .join(config.paths.templates.clone())
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

    // Clean up output directory
    let campfire_dir = base_dir.join(".campfire");
    let output_dir = campfire_dir.join(config.paths.target.clone());
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir).unwrap();
    }
    fs::create_dir(&output_dir).unwrap();
    fs::create_dir(output_dir.join("static")).unwrap();

    // Build global context
    let mut ctx = GeneratorContext {
        config,
        tera,
        base_dir,
        output_dir,
        posts: vec![],
    };

    // create posts and metadata for each entry
    for file in files {
        let post_context = create_post_metadata(&ctx, &file);
        ctx.posts.push((file.into(), post_context));
    }
    ctx.posts.sort_by(|(_, a), (_, b)| b.date.cmp(&a.date));

    // render individual posts
    let mut posts: Vec<(MarkdownFile, PostContext)> = Vec::new();
    for (file, post_context) in &ctx.posts {
        let post_context = generate_post_and_copy_assets(&ctx, post_context, &file);
        posts.push((file.clone(), post_context));
    }
    ctx.posts = posts;

    // render index & feed
    generate_index_and_feed(&ctx);

    copy_static_files(&ctx);

    run_post_build_command(&ctx, campfire_dir);
}

fn create_post_metadata(ctx: &GeneratorContext, file: &MarkdownFile) -> PostContext {
    let tags: Vec<String> = file
        .frontmatter
        .tags
        .iter()
        .map(|t| t.clone())
        .filter(|t| t.as_str() != &ctx.config.require_tag)
        .collect();
    let author = match &file.frontmatter.author {
        Some(author) => author,
        None => &ctx.config.author,
    }
    .clone();
    return PostContext {
        title: file.title(),
        tags,
        author,
        date: file
            .frontmatter
            .date
            .unwrap()
            .format("%Y-%m-%d")
            .to_string(),
        year: file.frontmatter.date.unwrap().year(),
        month: file.frontmatter.date.unwrap().month(),
        day: file.frontmatter.date.unwrap().day(),
        markdown: "".into(),
        original_file_name: file
            .path
            .strip_prefix(&ctx.base_dir)
            .unwrap()
            .to_str()
            .unwrap()
            .into(),
        relative_url: format!("{}/", file.slug(&ctx.base_dir)),
    };
}

fn generate_post_and_copy_assets(
    ctx: &GeneratorContext,
    post_context: &PostContext,
    file: &MarkdownFile,
) -> PostContext {
    let file_dir = ctx.output_dir.join(file.slug(&ctx.base_dir));
    let output_file = file_dir.join("index.html");

    let (html, assets) = &file.render_to_html(&ctx);
    let post_context = PostContext {
        markdown: html.into(),
        ..post_context.clone()
    };

    let mut context = Context::new();
    context.insert("post", &post_context);
    context.insert("base_url", &ctx.config.base_url);
    context.insert("site_title", &ctx.config.title());

    info!("Generating {}", file_dir.to_str().unwrap());

    let rendered = ctx.tera.render("post.html", &context).unwrap();
    fs::create_dir_all(&file_dir).expect("Failed to create directory");
    fs::write(output_file, rendered).expect("Failed to write output");

    for asset in assets {
        let asset_source_path = &ctx.base_dir.join(&asset.source);
        let asset_target_path = ctx.output_dir.join(&asset.target);
        debug!("  Copying asset {}", asset_target_path.to_str().unwrap());
        fs::copy(asset_source_path, asset_target_path).unwrap();
    }

    return post_context;
}

fn generate_index_and_feed(ctx: &GeneratorContext) {
    let posts: Vec<&PostContext> = ctx
        .posts
        .iter()
        .map(|(_, post_context)| post_context)
        .collect();
    let mut context = Context::new();
    context.insert("posts", &posts);
    // TODO pass a global context around and extend sub-contexts from it
    context.insert("base_url", &ctx.config.base_url);
    context.insert("site_title", &ctx.config.title());

    let index = ctx.tera.render("index.html", &context).unwrap();
    fs::write(&ctx.output_dir.join("index.html"), index).expect("Failed to write index");

    let feed = ctx.tera.render("feed.xml", &context);
    let feed = match feed {
        Ok(feed) => feed,
        Err(_) => Tera::one_off(include_str!("templates/feed.xml"), &context, true).unwrap(),
    };
    fs::write(&ctx.output_dir.join(&ctx.config.feed_path), feed).expect("Failed to write feed");
}

fn copy_static_files(ctx: &GeneratorContext) {
    let source = &ctx.base_dir.join(".campfire").join("static");
    if source.exists() {
        info!(
            "Copying static files from {}",
            source.as_path().to_str().unwrap()
        );
        let copied_files = dir::copy_recursively(source, &ctx.output_dir);
        info!("Copied {} files", copied_files);
    } else {
        warn!("Not copying static files, directory doesn't exist");
    }
}

fn run_post_build_command(ctx: &GeneratorContext, campfire_dir: PathBuf) {
    let post_build_command = &ctx
        .config
        .post_build_command
        .replace("{{target}}", &ctx.output_dir.to_str().unwrap());
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
