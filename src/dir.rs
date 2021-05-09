use crate::config::Config;
use crate::markdown::{read_markdown_file, MarkdownFile};
use std::fs;
use std::path::{Path, PathBuf};
use log::{debug};

pub fn find_all_markdown_files(base_directory: &Path, config: &Config) -> Vec<MarkdownFile> {
    let mut markdown_files = Vec::new();
    for entry in base_directory.read_dir().expect("Could not read directory") {
        if let Ok(entry) = entry {
            let file_name = entry.file_name().into_string().unwrap();
            if file_name.starts_with(".") || file_name.starts_with("_") {
                continue;
            }

            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    markdown_files.append(&mut find_all_markdown_files(&entry.path(), &config));
                } else if file_name.ends_with(".md") {
                    let markdown_file = read_markdown_file(entry.path().clone());
                    if let Some(markdown_file) = markdown_file {
                        if is_allowed(&markdown_file, &config) {
                            markdown_files.push(markdown_file);
                        }
                    }
                }
            } else {
                panic!("Couldn't get file type for {:?}", entry.path())
            }
        }
    }

    markdown_files
}

fn is_allowed(file: &MarkdownFile, config: &Config) -> bool {
    file.frontmatter.tags.contains(&config.require_tag)
}

pub fn copy_recursively(source: &PathBuf, target: &PathBuf) -> u32 {
    if !target.exists() {
        fs::create_dir(target).unwrap();
    }

    let mut count: u32 = 0;
    for file in source.read_dir().unwrap() {
        if let Ok(entry) = file {
            if let Ok(file_type) = entry.file_type() {
                let source_file = source.join(entry.file_name());
                let target_file = target.join(entry.file_name());
                if file_type.is_dir() {
                    count += copy_recursively(&source_file, &target_file);
                } else {
                    debug!(
                        "Copying {} to {}",
                        &source_file.to_str().unwrap(),
                        &target_file.to_str().unwrap()
                    );
                    fs::copy(source_file, target_file).unwrap();
                    count += 1;
                }
            } else {
                panic!("Couldn't get file type for {:?}", entry.path())
            }
        }
    }

    count
}
