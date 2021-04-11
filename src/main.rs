mod config;
mod dir;
mod markdown;

#[macro_use]
extern crate lazy_static;
use crate::config::read_config;
use std::path::Path;
use std::{env, fs};

fn main() {
    if std::env::args().len() != 2 {
        panic!("No base directory provided")
    }
    let base_directory = &env::args().nth(1).unwrap();
    let base_directory = Path::new(base_directory);
    let config = read_config(base_directory).expect("Could not read config");
    println!("{:?}", config);

    let files = dir::find_all_markdown_files(&base_directory, &config);
    println!("Generating {} files", files.len());

    let output_dir = base_directory
        .join(".campfire")
        .join(format!("{}", &config.name));
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir).unwrap();
    }
    fs::create_dir(&output_dir).unwrap();

    for file in files {
        let dest = file.render_to_html();
        let output_file = output_dir.join(format!("{}.html", file.slug(base_directory)));
        assert!(output_file.to_str().unwrap().contains(".campfire"));
        println!(
            "Writing {:?} -- {:?} {:?}",
            output_file, file.path, file.frontmatter
        );
        fs::write(output_file, &dest).expect("Failed to write output");
    }
    println!("Done.");
}
