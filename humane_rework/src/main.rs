use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, path::PathBuf, time::Instant};

use console::style;
use futures::stream::StreamExt;
use futures::{future::join_all, stream::FuturesUnordered};
use pagebrowse_lib::PagebrowseBuilder;
use schematic::color::owo::OwoColorize;
use segments::HumaneSegments;
use similar_string::find_best_similarity;
use tokio::fs::read_to_string;
use tokio::time::sleep;
use wax::Glob;

use crate::definitions::{
    register_assertions, register_instructions, register_retrievers, HumaneInstruction,
};
use crate::differ::diff_snapshots;
use crate::errors::{HumaneStepError, HumaneTestError, HumaneTestFailure};
use crate::options::configure;
use crate::parser::parse_segments;
use crate::universe::Universe;
use crate::{parser::parse_file, runner::run_humane_experiment, writer::write_yaml_snapshots};

mod civilization;
mod definitions;
mod differ;
mod errors;
mod options;
mod parser;
mod runner;
mod segments;
mod universe;
mod writer;

#[derive(Debug, Clone)]
pub struct HumaneTestFile {
    pub test: String,
    pub steps: Vec<HumaneTestStep>,
    pub original_source: String,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HumaneTestStepState {
    Dormant,
    Failed,
    Passed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HumaneTestStep {
    Ref {
        other_file: PathBuf,
        orig: String,
        state: HumaneTestStepState,
    },
    Instruction {
        step: HumaneSegments,
        args: HashMap<String, serde_json::Value>,
        orig: String,
        state: HumaneTestStepState,
    },
    Assertion {
        retrieval: HumaneSegments,
        assertion: HumaneSegments,
        args: HashMap<String, serde_json::Value>,
        orig: String,
        state: HumaneTestStepState,
    },
    Snapshot {
        snapshot: HumaneSegments,
        snapshot_content: Option<String>,
        args: HashMap<String, serde_json::Value>,
        orig: String,
        state: HumaneTestStepState,
    },
}

impl Display for HumaneTestStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use HumaneTestStep::*;

        match self {
            Instruction { orig, .. } | Assertion { orig, .. } => {
                write!(f, "{}", orig)
            }
            Ref { orig, .. } => {
                write!(f, "run steps from: {}", orig)
            }
            Snapshot { orig, .. } => {
                write!(f, "snapshot: {}", orig)
            }
        }
    }
}

impl HumaneTestStep {
    pub fn args_pretty(&self) -> String {
        let args = match self {
            HumaneTestStep::Instruction { args, .. } => Some(args),
            HumaneTestStep::Assertion { args, .. } => Some(args),
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

    pub fn state(&self) -> HumaneTestStepState {
        use HumaneTestStep::*;

        match self {
            Ref { state, .. }
            | Instruction { state, .. }
            | Assertion { state, .. }
            | Snapshot { state, .. } => state.clone(),
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
            let test_file = match parse_file(&i.unwrap(), p.clone()) {
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

    let all_retrievers = register_retrievers();
    let retriever_comparisons: Vec<_> = all_retrievers
        .keys()
        .map(|k| k.get_comparison_string())
        .collect();

    let all_assertions = register_assertions();
    let assertion_comparisons: Vec<_> = all_assertions
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
        retrievers: all_retrievers,
        retriever_comparisons,
        assertions: all_assertions,
        assertion_comparisons,
        ctx,
    });

    // let mut humanity = FuturesUnordered::new();

    let handle_res = |universe: Arc<Universe>,
                      (file, res): (HumaneTestFile, Result<(), HumaneTestError>)|
     -> Result<(), ()> {
        let log_err_preamble = || {
            println!("{}", style(&format!("✘ {}", &file.test)).red().bold());
            println!("{}", style("--- STEPS ---").on_yellow().bold());
            for step in &file.steps {
                use HumaneTestStepState::*;
                println!(
                    "{}",
                    match step.state() {
                        Dormant => style(format!("⦸ {step}")).dim(),
                        Failed => style(format!("✘ {step}")).red(),
                        Passed => style(format!("✓ {step}")).green(),
                    }
                );
            }
        };

        let output_doc = write_yaml_snapshots(&file.original_source, &file);

        match res {
            Ok(_) => {
                if output_doc == file.original_source {
                    let msg = format!("✓ {}", file.test);
                    println!("{}", msg.green());
                    Ok(())
                } else {
                    println!("{}", format!("⚠ {}", &file.test).yellow().bold());
                    println!("{}\n", "--- SNAPSHOT CHANGED ---".on_bright_yellow().bold());
                    diff_snapshots(&file.original_source, &output_doc);
                    println!(
                        "\n{}",
                        "--- END SNAPSHOT CHANGE ---".on_bright_yellow().bold()
                    );
                    println!(
                        "\n{}",
                        "Run in interactive mode to accept new snapshots\n"
                            .bright_red()
                            .bold()
                    );
                    Err(())
                }
            }
            Err(e) => {
                let log_err = || {
                    log_err_preamble();
                    println!("{}", "--- ERROR ---".on_yellow().bold());
                    println!("{}", &e.red());
                };

                let log_closest = |step_type: &str,
                                   original_segment_string: &str,
                                   user_segments: &HumaneSegments,
                                   comparisons: &Vec<String>| {
                    let comparator = user_segments.get_comparison_string();
                    let (best_match, _) = find_best_similarity(&comparator, comparisons)
                        .expect("Some comparisons should exist");
                    let parsed = parse_segments(&best_match)
                        .expect("strings were serialized so should always parse");

                    eprintln!(
                        "Unable to resolve: \"{}\"\n{step_type} \"{}\" was not found.",
                        original_segment_string.red(),
                        comparator.yellow(),
                    );

                    parsed
                };

                match &e.err {
                    HumaneStepError::External(ex) => match ex {
                        errors::HumaneInputError::NonexistentStep => {
                            log_err_preamble();
                            println!("{}", "--- ERROR ---".on_yellow().bold());
                            match e.step {
                                HumaneTestStep::Ref {
                                    other_file,
                                    orig,
                                    state,
                                } => todo!(),
                                HumaneTestStep::Instruction {
                                    step,
                                    args,
                                    orig,
                                    state,
                                } => {
                                    let closest = log_closest(
                                        "Instruction",
                                        &orig,
                                        &step,
                                        &universe.instruction_comparisons,
                                    );

                                    let (actual_segments, _) = universe
                                        .instructions
                                        .get_key_value(&closest)
                                        .expect("should exist in the global set");

                                    eprintln!(
                                        "Closest match: \"{}\"",
                                        style(actual_segments.get_as_string()).cyan()
                                    );
                                }
                                HumaneTestStep::Assertion {
                                    retrieval,
                                    assertion,
                                    args,
                                    orig,
                                    state,
                                } => {
                                    if !universe.retrievers.contains_key(&retrieval) {
                                        let closest = log_closest(
                                            "Retrieval",
                                            &orig,
                                            &retrieval,
                                            &universe.retriever_comparisons,
                                        );

                                        let (actual_segments, _) = universe
                                            .retrievers
                                            .get_key_value(&closest)
                                            .expect("should exist in the global set");

                                        eprintln!(
                                            "Closest match: \"{}\"",
                                            style(actual_segments.get_as_string()).cyan()
                                        );
                                    } else {
                                        let closest = log_closest(
                                            "Assertion",
                                            &orig,
                                            &assertion,
                                            &universe.assertion_comparisons,
                                        );

                                        let (actual_segments, _) = universe
                                            .assertions
                                            .get_key_value(&closest)
                                            .expect("should exist in the global set");

                                        eprintln!(
                                            "Closest match: \"{}\"",
                                            style(actual_segments.get_as_string()).cyan()
                                        );
                                    }
                                }
                                HumaneTestStep::Snapshot {
                                    snapshot,
                                    snapshot_content,
                                    args,
                                    orig,
                                    state,
                                } => todo!(),
                            }
                        }
                        _ => {
                            log_err();
                        }
                    },
                    _ => {
                        log_err();
                    }
                }
                Err(())
            }
        }
    };

    let semaphore = Arc::new(tokio::sync::Semaphore::new(universe.ctx.params.concurrency));

    let mut hands = vec![];

    for mut test in universe.tests.values().cloned() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let uni = Arc::clone(&universe);
        hands.push(tokio::spawn(async move {
            let res = run_humane_experiment(&mut test, Arc::clone(&uni)).await;
            let passed = handle_res(uni, (test, res)).is_ok();

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
        style(&format!("Passing tests: {}", passing)).cyan(),
        style(&format!("Failing tests: {}", failing)).cyan()
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
