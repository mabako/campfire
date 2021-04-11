use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    #[serde(rename = "require-tag")]
    pub require_tag: String,
}

pub fn read_config(base_directory: &Path) -> Result<Config, Box<dyn Error>> {
    let campfire_config = base_directory.join(".campfire").join("campfire.yaml");
    let reader = BufReader::new(File::open(campfire_config)?);
    Ok(serde_yaml::from_reader(reader)?)
}
