use clap::{arg, command, value_parser, Arg, ArgAction, ArgMatches, Command};
use schematic::{derive_enum, Config, ConfigEnum, ConfigLoader};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, path::PathBuf};

const CONFIGS: &[&str] = &["humane.json", "humane.yml", "humane.yaml", "humane.toml"];

pub fn configure() -> HumaneContext {
    let cli_matches = get_cli_matches();

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
        std::process::exit(1);
    }

    let mut loader = ConfigLoader::<HumaneParams>::new();
    for config in configs {
        if let Err(e) = loader.file(config) {
            eprintln!("Failed to load {config}:\n{e}");
            std::process::exit(1);
        }
    }

    match loader.load() {
        Err(e) => {
            eprintln!("Failed to initialize configuration: {e}");
            std::process::exit(1);
        }
        Ok(mut result) => {
            result.config.override_from_cli(cli_matches);

            match HumaneContext::load(result.config) {
                Ok(ctx) => ctx,
                Err(e) => {
                    eprintln!("Failed to initialize configuration");
                    std::process::exit(1);
                }
            }
        }
    }
}

fn get_cli_matches() -> ArgMatches {
    command!()
        .arg(
            arg!(
                -r --root <DIR> "The location from which to look for humane test files"
            )
            .required(false)
            .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            arg!(
                -c --concurrency <NUM> "How many tests should be run concurrently"
            )
            .required(false)
            .value_parser(value_parser!(usize)),
        )
        .arg(
            arg!(--placeholders <PAIRS> "Define placeholders for tests")
                .long_help("e.g. --placeholders key=value second_key=second_value")
                .required(false)
                .num_args(0..),
        )
        .arg(
            arg!(
                -v --verbose ... "Print verbose logging while running tests"
            )
            .action(clap::ArgAction::SetTrue),
        )
        .arg(
            arg!(
                --porcelain ... "Reduce logging to be stable"
            )
            .action(clap::ArgAction::SetTrue),
        )
        .arg(
            arg!(
                -i --interactive ... "Run humane in interactive mode"
            )
            .action(clap::ArgAction::SetTrue),
        )
        .get_matches()
}

#[derive(Config, Debug, Clone)]
#[config(rename_all = "snake_case")]
pub struct HumaneParams {
    /// The location from which to look for humane test files
    #[setting(env = "HUMANE_ROOT")]
    pub root: Option<PathBuf>,

    /// Print verbose logging while building. Does not impact the output files
    #[setting(env = "HUMANE_VERBOSE")]
    pub verbose: bool,

    /// Reduce logging to be stable
    #[setting(env = "HUMANE_PORCELAIN")]
    pub porcelain: bool,

    /// Run humane in interactive mode
    pub interactive: bool,

    /// How many tests should be run concurrently
    #[setting(env = "HUMANE_CONCURRENCY")]
    #[setting(default = 10)]
    pub concurrency: usize,

    /// What delimiter should be used when replacing placeholders
    #[setting(env = "HUMANE_PLACEHOLDER_DELIM")]
    #[setting(default = "%")]
    pub placeholder_delimiter: String,

    /// Placeholder keys, and the values they should be replaced with
    pub placeholders: HashMap<String, String>,
}

// The configuration object used internally
#[derive(Debug, Clone)]
pub struct HumaneContext {
    pub version: &'static str,
    pub working_directory: PathBuf,
    pub params: HumaneParams,
}

impl HumaneContext {
    fn load(mut config: HumaneParams) -> Result<Self, ()> {
        let working_directory = env::current_dir().unwrap();

        if let Some(root) = config.root.as_mut() {
            *root = working_directory.join(root.clone());
        }

        Ok(Self {
            working_directory,
            version: env!("CARGO_PKG_VERSION"),
            params: config,
        })
    }
}

impl HumaneParams {
    fn override_from_cli(&mut self, cli_matches: ArgMatches) {
        if cli_matches.get_flag("verbose") {
            self.verbose = true;
        }

        if cli_matches.get_flag("porcelain") {
            self.porcelain = true;
        }

        if cli_matches.get_flag("interactive") {
            self.interactive = true;
        }

        if let Some(root) = cli_matches.get_one::<PathBuf>("root") {
            self.root = Some(root.clone());
        }

        if let Some(concurrency) = cli_matches.get_one::<usize>("concurrency") {
            self.concurrency = *concurrency;
        }

        if let Some(placeholders) = cli_matches.get_many::<String>("placeholders") {
            for placeholder in placeholders {
                let Some((key, value)) = placeholder.split_once('=') else {
                    eprintln!("Error parsing --placeholders, expected a value of key=value but received {placeholder}");
                    std::process::exit(1);
                };

                self.placeholders.insert(key.into(), value.into());
            }
        }
    }
}
