use std::time::Instant;

use twelf::reexports::clap::CommandFactory;
use twelf::Layer;

use humane::options::{HumanHumaneConfig, RobotHumaneConfig};
use humane::Humane;

const CONFIGS: &[&str] = &["humane.json", "humane.yml", "humane.yaml", "humane.toml"];

#[tokio::main]
async fn main() {
    let start = Instant::now();

    let matches = HumanHumaneConfig::command().get_matches();

    let mut config_layers = vec![];

    let configs: Vec<&str> = CONFIGS
        .iter()
        .filter(|c| std::path::Path::new(c).exists())
        .cloned()
        .collect();
    if configs.len() > 1 {
        eprintln!(
            "Found multiple possible config files: [{}]",
            configs.join(", ")
        );
        eprintln!("Humane only supports loading one configuration file format, please ensure only one file exists.");
        return;
    }

    for config in configs {
        let layer_fn = if config.ends_with("json") {
            Layer::Json
        } else if config.ends_with("toml") {
            Layer::Toml
        } else if config.ends_with("yaml") || config.ends_with("yml") {
            Layer::Yaml
        } else {
            panic!("Unknown config file format");
        };
        config_layers.push(layer_fn(config.into()));
    }

    config_layers.push(Layer::Env(Some("HUMANE_".to_string())));
    config_layers.push(Layer::Clap(matches));

    match HumanHumaneConfig::with_layers(&config_layers) {
        Ok(config) => {
            if let Ok(options) = RobotHumaneConfig::load(config.clone()) {
                let mut humane = Humane::new(options);

                humane.go().await;

                let duration = start.elapsed();
                println!(
                    "Finished in {}.{} seconds",
                    duration.as_secs(),
                    duration.subsec_millis()
                );
            }
        }
        Err(e) => {
            eprintln!("Error loading Humane config:");
            match e {
                twelf::Error::Io(e) => {
                    eprintln!("{}", e);
                }
                twelf::Error::Envy(e) => {
                    eprintln!("{}", e);
                }
                twelf::Error::Json(e) => {
                    eprintln!("{}", e);
                }
                twelf::Error::Toml(e) => {
                    eprintln!("{}", e);
                }
                twelf::Error::Yaml(e) => {
                    eprintln!("{}", e);
                }
                twelf::Error::Deserialize(e) => {
                    eprintln!("{}", e);
                }
                _ => {
                    eprintln!("Unknown Error");
                }
            }
        }
    }
}
