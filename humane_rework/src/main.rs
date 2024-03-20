use std::fmt::Display;
use std::{collections::HashMap, path::PathBuf, time::Instant};

use futures::stream::StreamExt;
use futures::{future::join_all, stream::FuturesUnordered};
use instructions::HumaneSegments;
use similar_string::find_best_similarity;
use tokio::fs::read_to_string;
use wax::Glob;

use crate::errors::{HumaneStepError, HumaneTestError, HumaneTestFailure};
use crate::instructions::register_instructions;
use crate::parser::parse_instruction;
use crate::{parser::parse_file, runner::run_humane_experiment, writer::write_yaml_snapshots};

mod civilization;
mod errors;
mod instructions;
mod parser;
mod runner;
mod writer;

#[derive(Debug, Clone)]
pub struct HumaneTestFile {
    pub test: String,
    pub setup: Vec<HumaneTestStep>,
    pub steps: Vec<HumaneTestStep>,
}

#[derive(Debug, Clone)]
pub enum HumaneTestStep {
    Ref {
        other_file: PathBuf,
        orig: String,
    },
    Step {
        step: HumaneSegments,
        args: HashMap<String, serde_json::Value>,
        orig: String,
    },
    Snapshot {
        snapshot: HumaneSegments,
        snapshot_content: Option<String>,
        args: HashMap<String, serde_json::Value>,
        orig: String,
    },
}

impl Display for HumaneTestStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use HumaneTestStep::*;

        match self {
            Ref { orig, .. } | Step { orig, .. } | Snapshot { orig, .. } => write!(f, "{}", orig),
        }
    }
}

#[tokio::main]
async fn main() {
    let start = Instant::now();

    let glob = Glob::new("**/*.humane.yml").expect("Valid glob");
    let walker = glob.walk(".").flatten();

    let loaded_files = walker
        .map(|entry| {
            let file = entry.path().to_path_buf();
            async { (file.clone(), read_to_string(file).await) }
        })
        .collect::<Vec<_>>();

    let files = join_all(loaded_files).await;
    let errors: Vec<_> = files
        .iter()
        .filter_map(|(path, inner)| {
            if let Err(e) = inner {
                Some(format!("Failed to load {}: {e}", path.to_string_lossy()))
            } else {
                None
            }
        })
        .collect();
    if !errors.is_empty() {
        eprintln!("Humane failed to load some files:");
        for e in errors {
            eprintln!("  • {e}");
        }
        std::process::exit(1);
    }

    let mut errors = vec![];
    let all_tests: HashMap<_, _> = files
        .into_iter()
        .filter_map(|(p, i)| {
            let test_file = match parse_file(&i.unwrap()) {
                Ok(f) => f,
                Err(e) => {
                    errors.push(e);
                    return None;
                }
            };
            Some((p, test_file))
        })
        .collect();

    if !errors.is_empty() {
        eprintln!("Humane failed to parse some files:");
        for e in errors {
            eprintln!("  • {e}");
        }
        std::process::exit(1);
    }

    let all_instructions = register_instructions();

    let instruction_comparisons: Vec<_> = all_instructions
        .keys()
        .map(|k| k.get_comparison_string())
        .collect();

    let mut humanity = FuturesUnordered::new();

    let handle_res = |(file, res): (&HumaneTestFile, Result<(), HumaneTestError>)| match res {
        Ok(h) => println!("----> Test succeeded: {}", file.test),
        Err(e) => match &e.err {
            HumaneStepError::External(ex) => match ex {
                errors::HumaneInputError::NonexistentStep => match e.step {
                    HumaneTestStep::Ref { other_file, orig } => todo!(),
                    HumaneTestStep::Step { step, args, orig } => {
                        let instruction_comparator = step.get_comparison_string();
                        let (best_match, _) =
                            find_best_similarity(&instruction_comparator, &instruction_comparisons)
                                .expect("Some instructions should exist");
                        let parsed = parse_instruction(&best_match)
                            .expect("strings were serialized so shoudl always parse");

                        let (actual_instruction, _) = all_instructions
                            .get_key_value(&parsed)
                            .expect("should exist in the global set");

                        eprintln!(
                            "Step \"{orig}\" was not found.\nLooked for \"{instruction_comparator}\"\nClosest matching step: \"{}\"",
                            actual_instruction.get_as_string()
                        );
                    }
                    HumaneTestStep::Snapshot {
                        snapshot,
                        snapshot_content,
                        args,
                        orig,
                    } => todo!(),
                },
                _ => eprintln!("!!!!> Test {} failed:\n{e}", file.test),
            },
            _ => eprintln!("!!!!> Test {} failed:\n{e}", file.test),
        },
    };

    for test_file in all_tests.values() {
        humanity.push(async {
            (
                &*test_file,
                run_humane_experiment(test_file, &all_tests, &all_instructions).await,
            )
        });

        // TODO: Wire up to concurrency option
        if humanity.len() == 10 {
            if let Some(res) = humanity.next().await {
                handle_res(res);
            }
        }
    }

    while let Some(res) = humanity.next().await {
        handle_res(res);
    }

    return;
    // let base_dir = self.tmp_file_path(".");
    // let walk = glob.walk(&base_dir).flatten();
    // let entries: Vec<String> = walk
    //     .filter_map(|entry| {
    //         let file = entry
    //             .path()
    //             .strip_prefix(&base_dir)
    //             .expect("Valid file path");
    //         let indentation = "  ".repeat(file.components().count().saturating_sub(1));
    //         file.file_name().map(|filename| {
    //             format!(
    //                 "| {}{}",
    //                 indentation,
    //                 filename.to_str().expect("Valid filename utf8")
    //             )
    //         })
    //     })
    //     .collect();

    let file = read_to_string("example-test.humane.yml").await.unwrap();

    let test = parse_file(&file);

    println!("{:#?}", test);

    let Ok(mut test) = test else {
        panic!("Could not parse that file");
    };

    for step in test.steps.iter_mut() {
        match step {
            HumaneTestStep::Snapshot {
                snapshot,
                snapshot_content,
                args,
                orig,
            } => {
                *snapshot_content = Some("Wahoooo\nmy snapshot content\ngoes here!!".to_string());
            }
            _ => {}
        }
    }

    let output_doc = write_yaml_snapshots(&file, &test);
    println!("---\n{output_doc}\n---");

    let duration = start.elapsed();
    println!(
        "Finished in {}.{:03} seconds",
        duration.as_secs(),
        duration.subsec_millis()
    );
}
