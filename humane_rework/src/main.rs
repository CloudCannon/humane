use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, path::PathBuf, time::Instant};

use console::style;
use futures::stream::StreamExt;
use futures::{future::join_all, stream::FuturesUnordered};
use instructions::HumaneSegments;
use pagebrowse_lib::PagebrowseBuilder;
use similar_string::find_best_similarity;
use tokio::fs::read_to_string;
use tokio::time::sleep;
use wax::Glob;

use crate::errors::{HumaneStepError, HumaneTestError, HumaneTestFailure};
use crate::instructions::register_instructions;
use crate::options::configure;
use crate::parser::parse_instruction;
use crate::universe::Universe;
use crate::{parser::parse_file, runner::run_humane_experiment, writer::write_yaml_snapshots};

mod civilization;
mod errors;
mod instructions;
mod options;
mod parser;
mod runner;
mod universe;
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

impl HumaneTestStep {
    pub fn args_pretty(&self) -> String {
        let args = match self {
            HumaneTestStep::Step { args, .. } => Some(args),
            HumaneTestStep::Snapshot { args, .. } => Some(args),
            _ => None,
        };

        if let Some(args) = args {
            let res = format!("{}", serde_yaml::to_string(&args).unwrap());
            if res.trim() == "{}" {
                String::new()
            } else {
                res
            }
        } else {
            String::new()
        }
    }
}

#[tokio::main]
async fn main() {
    let ctx = configure();

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

    let pagebrowser = PagebrowseBuilder::new(ctx.params.concurrency)
        .visible(false)
        .manager_path(format!(
            "{}/../../pagebrowse/target/debug/pagebrowse_manager",
            env!("CARGO_MANIFEST_DIR")
        ))
        .build()
        .expect("Can't build the pagebrowser");

    let universe = Arc::new(Universe {
        pagebrowser: Arc::new(pagebrowser),
        tests: all_tests,
        instructions: all_instructions,
        instruction_comparisons,
        ctx,
    });

    // let mut humanity = FuturesUnordered::new();

    let handle_res = |universe: Arc<Universe>,
                      (file, res): (
        HumaneTestFile,
        Result<Vec<String>, (Vec<String>, HumaneTestError)>,
    )| match res {
        Ok(_logs) => {
            let msg = format!("✅ {}", file.test);
            println!("{}", style(msg).green());
        }
        Err((logs, e)) => {
            let log_err = || {
                println!(
                    "{}",
                    style(&format!("\n### Test failed: {}", &file.test))
                        .red()
                        .bold()
                );
                println!("{}", style("--- STEP LOGS ---").on_yellow().bold());
                for log in logs {
                    println!("{log}");
                }
                println!("{}", style("--- ERROR ---").on_yellow().bold());
                println!("{}", style(&e).red());
            };

            match &e.err {
                HumaneStepError::External(ex) => match ex {
                    errors::HumaneInputError::NonexistentStep => match e.step {
                        HumaneTestStep::Ref { other_file, orig } => todo!(),
                        HumaneTestStep::Step { step, args, orig } => {
                            let instruction_comparator = step.get_comparison_string();
                            let (best_match, _) = find_best_similarity(
                                &instruction_comparator,
                                &universe.instruction_comparisons,
                            )
                            .expect("Some instructions should exist");
                            let parsed = parse_instruction(&best_match)
                                .expect("strings were serialized so shoudl always parse");

                            let (actual_instruction, _) = universe
                                .instructions
                                .get_key_value(&parsed)
                                .expect("should exist in the global set");

                            eprintln!(
                            "Step \"{}\" was not found. (no match for \"{}\")\nClosest matching step: \"{}\"",
                            style(orig).red(),
                            style(instruction_comparator).yellow(),
                            style(actual_instruction.get_as_string()).cyan()
                        );
                        }
                        HumaneTestStep::Snapshot {
                            snapshot,
                            snapshot_content,
                            args,
                            orig,
                        } => todo!(),
                    },
                    _ => {
                        log_err();
                    }
                },
                _ => {
                    log_err();
                }
            }
        }
    };

    let semaphore = Arc::new(tokio::sync::Semaphore::new(universe.ctx.params.concurrency));

    let mut hands = vec![];

    for test in universe.tests.values().cloned() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let uni = Arc::clone(&universe);
        hands.push(tokio::spawn(async move {
            let res = run_humane_experiment(&test, Arc::clone(&uni)).await;
            let passed = res.is_ok();

            handle_res(uni, (test, res));

            drop(permit);

            if passed {
                Ok(())
            } else {
                Err(())
            }
        }));
    }

    let results = join_all(hands)
        .await
        .into_iter()
        .map(|outer_err| match outer_err {
            Ok(Ok(_)) => Ok(()),
            _ => Err(()),
        })
        .collect::<Vec<_>>();

    // for step in test.steps.iter_mut() {
    //     match step {
    //         HumaneTestStep::Snapshot {
    //             snapshot,
    //             snapshot_content,
    //             args,
    //             orig,
    //         } => {
    //             *snapshot_content = Some("Wahoooo\nmy snapshot content\ngoes here!!".to_string());
    //         }
    //         _ => {}
    //     }
    // }

    // let output_doc = write_yaml_snapshots(&file, &test);
    // println!("---\n{output_doc}\n---");

    let duration = start.elapsed();
    let duration = format!(
        "{}.{:03} seconds",
        duration.as_secs(),
        duration.subsec_millis()
    );

    let failing = results.iter().filter(|r| r.is_err()).count();
    let passing = results.iter().filter(|r| r.is_ok()).count();

    println!(
        "\n{}\n{}",
        style(&format!("Passing tests: {}", failing)).cyan(),
        style(&format!("Failing tests: {}", passing)).cyan()
    );

    if failing > 0 {
        println!(
            "{}",
            style(&format!("\nSome tests failed in {}", duration)).red()
        );
        std::process::exit(1);
    } else {
        println!(
            "{}",
            style(&format!("\nAll tests passed in {}", duration)).green()
        );
    }
}
