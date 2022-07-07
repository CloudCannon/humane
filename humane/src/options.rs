use anyhow::Result;
use clap::Parser;
use std::{env, path::PathBuf};
use twelf::config;

#[config]
#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct HumanHumaneConfig {
    #[clap(
        long,
        help = "Where to load test files from. Defaults to detecting anywhere below the current directory"
    )]
    #[clap(required = false)]
    #[serde(default = "defaults::default_test_files")]
    pub test_file_root: String,
}

mod defaults {
    pub fn default_test_files() -> String {
        ".".into()
    }
}

// The configuration object used internally
#[derive(Debug)]
pub struct RobotHumaneConfig {
    pub test_file_root: PathBuf,
    pub version: &'static str,
}

impl RobotHumaneConfig {
    pub fn load(config: HumanHumaneConfig) -> Result<Self> {
        Ok(Self {
            test_file_root: PathBuf::from(config.test_file_root),
            version: env!("CARGO_PKG_VERSION"),
        })
    }
}
