use actix_web::dev::ServerHandle;
use cucumber::gherkin::Table;
use portpicker::pick_unused_port;
use std::collections::HashMap;
use std::convert::Infallible;
use std::io::{Read, Write};
use std::process::Command;
use std::str::from_utf8;
use std::{fs, path::PathBuf};
use tempfile::tempdir;
use tokio::task::JoinHandle;
use wax::Glob;

use async_trait::async_trait;
use browser::BrowserTester;
use cucumber::{World, WorldInit};

mod browser;
mod steps;

#[derive(Debug)]
struct CommandOutput {
    stdout: String,
    stderr: String,
}

#[derive(Debug, Default, WorldInit)]
pub struct Civilization {
    tmp_dir: Option<tempfile::TempDir>,
    last_command_output: Option<CommandOutput>,
    browser: Option<BrowserTester>,
    assigned_server_port: Option<u16>,
    threads: Vec<JoinHandle<Result<(), std::io::Error>>>,
    handles: Vec<ServerHandle>,
    env_vars: HashMap<String, String>,
}

impl Civilization {
    pub async fn shutdown(&mut self) {
        for handle in &self.handles {
            handle.stop(false).await;
        }
        for thread in &self.threads {
            thread.abort();
        }
    }
}

impl Civilization {
    fn ensure_port(&mut self) -> u16 {
        if self.assigned_server_port.is_none() {
            self.assigned_server_port = pick_unused_port();
        }
        self.assigned_server_port.expect("No port was available")
    }
    fn purge_port(&mut self) {
        self.assigned_server_port = None;
    }
    async fn ensure_browser(&mut self) -> &mut BrowserTester {
        if self.browser.is_none() {
            self.browser = Some(BrowserTester::new().await);
        }
        self.browser.as_mut().unwrap()
    }

    fn tmp_dir(&mut self) -> PathBuf {
        if self.tmp_dir.is_none() {
            self.tmp_dir = Some(tempdir().expect("testing on a system with a temp dir"));
        }
        self.tmp_dir
            .as_ref()
            .expect("just created")
            .path()
            .to_path_buf()
    }

    fn tmp_file_path(&mut self, filename: &str) -> PathBuf {
        let tmp_dir = self.tmp_dir();
        tmp_dir.join(PathBuf::from(filename))
    }

    fn write_file(&mut self, filename: &str, contents: &str, gzipped: bool) {
        let file_path = self.tmp_file_path(filename);
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();

        let mut file = std::fs::File::create(&file_path).unwrap();
        if gzipped {
            let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
            gz.write_all(contents.as_bytes()).expect("Gzip failed");
            file.write_all(&gz.finish().expect("Gzip failed"))
                .expect("Write failed");
        } else {
            file.write_all(contents.as_bytes()).unwrap();
        }
    }

    fn read_file(&mut self, filename: &str) -> String {
        let file_path = self.tmp_file_path(filename);
        let mut file = std::fs::File::open(&file_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    }

    fn get_file_tree(&mut self) -> String {
        let glob = Glob::new("**/*").expect("Valid glob");
        let base_dir = self.tmp_file_path(".");
        let walk = glob.walk(&base_dir).flatten();
        let entries: Vec<String> = walk
            .filter_map(|entry| {
                let file = entry
                    .path()
                    .strip_prefix(&base_dir)
                    .expect("Valid file path");
                let indentation = "  ".repeat(file.components().count().saturating_sub(1));
                file.file_name().map(|filename| {
                    format!(
                        "| {}{}",
                        indentation,
                        filename.to_str().expect("Valid filename utf8")
                    )
                })
            })
            .collect();
        entries.join("\n")
    }

    fn assert_file_exists(&mut self, filename: &str) {
        if !self.check_file_exists(filename) {
            panic!(
                "\"{}\" does not exist in the tree:\n-----\n{}\n-----\n",
                filename,
                self.get_file_tree()
            );
        }
    }

    fn assert_file_doesnt_exist(&mut self, filename: &str) {
        if self.check_file_exists(filename) {
            panic!(
                "\"{}\" should not exist but does in the tree:\n-----\n{}\n-----\n",
                filename,
                self.get_file_tree()
            );
        }
    }

    fn check_file_exists(&mut self, filename: &str) -> bool {
        self.tmp_file_path(filename).exists()
    }

    fn set_env(&mut self, options: Option<&Table>) {
        if let Some(options) = options {
            for row in &options.rows {
                self.env_vars.insert(
                    row.get(0).cloned().unwrap_or_default(),
                    row.get(1).cloned().unwrap_or_default(),
                );
            }
        }
    }

    fn run_command(&mut self, options: Option<&Table>) {
        let binary = std::env::var("TEST_BINARY").unwrap_or_else(|_| {
            panic!("No binary supplied — please provide a TEST_BINARY environment variable");
        });

        let tmp_dir = if self.tmp_dir.is_some() {
            Some(self.tmp_dir().to_str().expect("Invalid utf-8").to_string())
        } else {
            None
        };
        let process_value = |str: &str| {
            if str.contains("{{humane_temp_dir}}") {
                str.replace("{{humane_temp_dir}}", tmp_dir.as_ref().expect("No tmp dir"))
            } else {
                str.to_string()
            }
        };

        let cli = build_command(&binary, None, options, process_value);
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .current_dir(self.tmp_dir())
            .arg(&cli.replace(std::path::MAIN_SEPARATOR, "/"));

        for (key, val) in &self.env_vars {
            command.env(key, val);
        }

        let output = command.output().expect("Failed to run binary");
        self.last_command_output = Some(CommandOutput {
            stdout: from_utf8(&output.stdout).unwrap_or("failed utf8").into(),
            stderr: from_utf8(&output.stderr).unwrap_or("failed utf8").into(),
        });
    }
}

/// `cucumber::World` needs to be implemented so this World is accepted in `Steps`
#[async_trait(?Send)]
impl World for Civilization {
    // We require some error type
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self::default())
    }
}

struct BinaryCommand(String);

impl BinaryCommand {
    fn add_flag(&mut self, flag: &str) {
        self.0 = format!("{} {}", self.0, flag);
    }

    fn consume(self) -> String {
        self.0
    }
}

fn build_command<F: Fn(&str) -> String>(
    binary: &str,
    subcommand: Option<&str>,
    options: Option<&Table>,
    process: F,
) -> String {
    let cwd = std::env::current_dir().unwrap();
    let binary_path = cwd.join(PathBuf::from(binary));
    let binary_path = binary_path.to_str().unwrap();

    let mut command = match subcommand {
        Some(subcommand) => BinaryCommand(format!("{} {}", binary_path, subcommand)),
        None => BinaryCommand(binary_path.into()),
    };

    if let Some(options) = options {
        for row in &options.rows {
            command.add_flag(&process(&row[0]));
        }
    }

    command.consume()
}
