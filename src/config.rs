use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    pub author: String,
    #[serde(rename = "require-tag")]
    pub require_tag: String,
    #[serde(rename = "base-url", default)]
    pub base_url: String,
    #[serde(rename = "post-build", default)]
    pub post_build_command: String,
    #[serde(rename = "feed-path", default = "default_feed_path")]
    pub feed_path: PathBuf,
    #[serde(default)]
    pub paths: Paths,
}

impl Config {
    pub fn title(&self) -> String {
        if self.title != "" {
            self.title.clone()
        } else {
            self.name.clone()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Paths {
    #[serde(default = "default_target_path")]
    pub target: PathBuf,
    #[serde(default = "default_template_path")]
    pub templates: PathBuf,
}

impl Default for Paths {
    fn default() -> Self {
        Paths {
            target: default_target_path(),
            templates: default_template_path(),
        }
    }
}

fn default_template_path() -> PathBuf {
    PathBuf::from("templates")
}

fn default_target_path() -> PathBuf {
    PathBuf::from("out")
}

fn default_feed_path() -> PathBuf {
    PathBuf::from("feed.xml")
}

pub fn read_config(config_file: &PathBuf) -> Result<Config, Box<dyn Error>> {
    let reader = BufReader::new(File::open(config_file)?);
    Ok(serde_yaml::from_reader(reader)?)
}
