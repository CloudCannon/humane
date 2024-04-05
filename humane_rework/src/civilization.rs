use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::Command,
    str::from_utf8,
    sync::Arc,
};

use actix_web::dev::ServerHandle;
use pagebrowse_lib::{Pagebrowser, PagebrowserWindow};
use portpicker::pick_unused_port;
use tempfile::tempdir;
use tokio::task::JoinHandle;
use wax::Glob;

use crate::universe::Universe;

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
}

pub struct Civilization<'u> {
    pub tmp_dir: Option<tempfile::TempDir>,
    pub last_command_output: Option<CommandOutput>,
    pub assigned_server_port: Option<u16>,
    pub window: Option<PagebrowserWindow>,
    pub threads: Vec<JoinHandle<Result<(), std::io::Error>>>,
    pub handles: Vec<ServerHandle>,
    pub env_vars: HashMap<String, String>,
    pub universe: Arc<Universe<'u>>,
}

impl<'u> Civilization<'u> {
    pub async fn shutdown(&mut self) {
        for handle in &self.handles {
            handle.stop(false).await;
        }
        for thread in &self.threads {
            thread.abort();
        }
    }
}

impl<'u> Civilization<'u> {
    pub fn ensure_port(&mut self) -> u16 {
        if self.assigned_server_port.is_none() {
            self.assigned_server_port = pick_unused_port();
        }
        self.assigned_server_port.expect("No port was available")
    }
    pub fn purge_port(&mut self) {
        self.assigned_server_port = None;
    }

    pub fn tmp_dir(&mut self) -> PathBuf {
        if self.tmp_dir.is_none() {
            self.tmp_dir = Some(tempdir().expect("testing on a system with a temp dir"));
        }
        self.tmp_dir
            .as_ref()
            .expect("just created")
            .path()
            .to_path_buf()
    }

    pub fn tmp_file_path(&mut self, filename: &str) -> PathBuf {
        let tmp_dir = self.tmp_dir();
        tmp_dir.join(PathBuf::from(filename))
    }

    pub fn write_file(&mut self, filename: &str, contents: &str) {
        let file_path = self.tmp_file_path(filename);
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();

        // let contents = self.process_substitutions(contents);

        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
    }

    pub fn read_file(&mut self, filename: &str) -> String {
        let file_path = self.tmp_file_path(filename);
        let mut file = std::fs::File::open(&file_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    }

    pub fn get_file_tree(&mut self) -> String {
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

    pub fn assert_file_exists(&mut self, filename: &str) {
        if !self.check_file_exists(filename) {
            panic!(
                "\"{}\" does not exist in the tree:\n-----\n{}\n-----\n",
                filename,
                self.get_file_tree()
            );
        }
    }

    pub fn assert_file_doesnt_exist(&mut self, filename: &str) {
        if self.check_file_exists(filename) {
            panic!(
                "\"{}\" should not exist but does in the tree:\n-----\n{}\n-----\n",
                filename,
                self.get_file_tree()
            );
        }
    }

    pub fn check_file_exists(&mut self, filename: &str) -> bool {
        self.tmp_file_path(filename).exists()
    }

    pub fn set_env(&mut self, options: ()) {
        todo!()
    }

    pub fn run_command(&mut self, options: ()) {
        let binary = std::env::var("TEST_BINARY").unwrap_or_else(|_| {
            panic!("No binary supplied — please provide a TEST_BINARY environment variable");
        });

        let cli = build_command(&binary, None, options);
        self.run_custom(cli);
    }

    pub fn run_custom<S: AsRef<str>>(&mut self, cmd: S) {
        let processed_cmd = cmd.as_ref(); //self.process_substitutions(cmd);

        let mut command = Command::new("sh");
        command
            .arg("-c")
            .current_dir(self.tmp_dir())
            .arg(&processed_cmd.replace(std::path::MAIN_SEPARATOR, "/"));

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

struct BinaryCommand(String);

impl BinaryCommand {
    fn add_flag(&mut self, flag: &str) {
        self.0 = format!("{} {}", self.0, flag);
    }

    fn consume(self) -> String {
        self.0
    }
}

fn build_command(binary: &str, subcommand: Option<&str>, options: ()) -> String {
    let cwd = std::env::current_dir().unwrap();
    let binary_path = cwd.join(PathBuf::from(binary));
    let binary_path = binary_path.to_str().unwrap();

    let mut command = match subcommand {
        Some(subcommand) => BinaryCommand(format!("{} {}", binary_path, subcommand)),
        None => BinaryCommand(binary_path.into()),
    };

    todo!();
    // if let Some(options) = options {
    //     for row in &options.rows {
    //         command.add_flag(&row[0]);
    //     }
    // }

    command.consume()
}
