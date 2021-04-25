use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    #[serde(rename = "require-tag")]
    pub require_tag: String,
    #[serde(rename = "template-path", default = "default_template_path")]
    pub template_path: PathBuf,
    #[serde(rename = "target-path", default = "default_target_path")]
    pub target_path: PathBuf,
    #[serde(rename = "base-url", default)]
    pub base_url: String,
    #[serde(rename = "post-build", default)]
    pub post_build_command: String,
}

fn default_template_path() -> PathBuf {
    PathBuf::from("templates")
}

fn default_target_path() -> PathBuf {
    PathBuf::from("out")
}

pub fn read_config(config_file: &PathBuf) -> Result<Config, Box<dyn Error>> {
    let reader = BufReader::new(File::open(config_file)?);
    Ok(serde_yaml::from_reader(reader)?)
}
