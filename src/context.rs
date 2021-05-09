use crate::config::Config;
use crate::markdown::MarkdownFile;
use serde::Serialize;
use std::path::PathBuf;
use tera::Tera;

#[derive(Serialize, Clone)]
pub struct PostContext {
    pub title: String,
    pub tags: Vec<String>,
    pub author: String,
    pub original_file_name: String,
    pub relative_url: String,

    pub date: String,
    pub year: i32,
    pub month: u32,
    pub day: u32,

    pub markdown: String,
}

pub struct GeneratorContext {
    pub config: Config,
    pub tera: Tera,
    pub base_dir: PathBuf,
    pub output_dir: PathBuf,
    pub posts: Vec<(MarkdownFile, PostContext)>,
}
