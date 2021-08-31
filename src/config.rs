use serde::{Serialize, Deserialize};
use std::{env, fs};
use std::path::{PathBuf};
use log::{warn, info};
use std::error::Error;
use crate::Result;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub graphics: Graphics,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Graphics {
    pub use_low_power_gpu: bool,
    pub sample_count: u32,
    pub tolerance: f32,
}

pub fn get_config_file() -> Result<PathBuf> {
    let exe_path = env::current_exe()?;
    Ok(exe_path.with_extension("toml"))
}

fn load_config_file() -> Result<Config> {
    let config_file = get_config_file()?;
    // if config_file.exists() {
    let conf_str = fs::read_to_string(&config_file)?;
    let config = toml::from_str(conf_str.as_str())?;
    return Ok(config);
}

fn save_config_file(config: &Config) -> Result<()> {
    let config_file = get_config_file()?;
    let conf_str = toml::to_string_pretty(&config)?;
    fs::write(&config_file, conf_str)?;
    Ok(())
}

pub fn load_config() -> Config {
    match load_config_file() {
        Ok(config) => {
            info!("Loaded configuration from ruzzle.toml");
            config
        },
        Err(err) => {
            warn!("Failed to parse config file: {}", err);
            info!("Using default configuration");
            let config = Config{
                graphics: Graphics {
                    sample_count: 4,
                    tolerance: 0.02,
                    use_low_power_gpu: true,
                }
            };
            save_config_file(&config);
            config
        }
    }
}