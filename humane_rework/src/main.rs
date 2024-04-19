use std::collections::BTreeMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, path::PathBuf, time::Instant};

use console::{style, Term};
use futures::stream::StreamExt;
use futures::{future::join_all, stream::FuturesUnordered};
use normalize_path::NormalizePath;
use pagebrowse_lib::PagebrowseBuilder;
use parser::HumaneFileType;
use schematic::color::owo::OwoColorize;
use segments::HumaneSegments;
use similar_string::{compare_similarity, find_best_similarity};
use tokio::fs::read_to_string;
use tokio::time::sleep;
use wax::Glob;

use crate::definitions::{
    register_assertions, register_instructions, register_retrievers, HumaneInstruction,
};
use crate::differ::diff_snapshots;
use crate::errors::{HumaneInputError, HumaneStepError, HumaneTestError, HumaneTestFailure};
use crate::interactive::{confirm_snapshot, get_run_mode, question, RunMode};
use crate::logging::log_step_runs;
use crate::options::configure;
use crate::parser::parse_segments;
use crate::universe::Universe;
use crate::{
    parser::parse_file, runner::run_humane_experiment, snapshot_writer::write_yaml_snapshots,
};

mod civilization;
mod definitions;
mod differ;
mod errors;
mod interactive;
mod logging;
mod options;
mod parser;
mod runner;
mod segments;
mod snapshot_writer;
mod universe;

#[derive(Debug, Clone)]
pub struct HumaneTestFile {
    pub name: String,
    r#type: HumaneFileType,
    pub steps: Vec<HumaneTestStep>,
    pub original_source: String,
    pub file_path: String,
    pub file_directory: String,
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
        other_file: String,
        orig: String,
        hydrated_steps: Option<Vec<HumaneTestStep>>,
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

fn closest_strings<'o>(target: &String, options: &'o Vec<String>) -> Vec<(&'o String, f64)> {
    let mut scores = options
        .iter()
        .map(|s| (s, compare_similarity(target, s)))
        .collect::<Vec<_>>();

    scores.sort_by(|a, b| {
        b.partial_cmp(a)
            .expect("similarities should not be NaN or Infinity")
    });

    scores
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

    let mut names_thus_far: Vec<(String, String)> = vec![];

    let mut errors = vec![];
    let all_tests: BTreeMap<_, _> = files
        .into_iter()
        .filter_map(|(p, i)| {
            let test_file = match parse_file(&i.unwrap(), p.clone()) {
                Ok(f) => {
                    if let Some((_, other_path)) = names_thus_far.iter().find(|(n, _)| *n == f.name)
                    {
                        errors.push(HumaneInputError::DuplicateName {
                            path_one: other_path.to_string(),
                            path_two: p.to_string_lossy().to_string(),
                            name: f.name.clone(),
                        });
                        return None;
                    }
                    names_thus_far.push((f.name.clone(), p.to_string_lossy().to_string()));
                    f
                }
                Err(e) => {
                    errors.push(e);
                    return None;
                }
            };
            Some((p.normalize().to_string_lossy().into_owned(), test_file))
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
        .init_script(include_str!("./definitions/browser/init.js").to_string())
        .build()
        .await
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

    let run_mode = if universe.ctx.params.interactive && !universe.ctx.params.all {
        match get_run_mode(&universe) {
            Ok(mode) => mode,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    } else {
        RunMode::All
    };

    enum HoldingError {
        TestFailure,
        SnapFailure { out: String },
    }

    let handle_res = |universe: Arc<Universe>,
                      (file, res): (&HumaneTestFile, Result<(), HumaneTestError>),
                      started_at: Instant|
     -> Result<(), HoldingError> {
        let dur = if universe.ctx.params.porcelain {
            "".to_string()
        } else {
            let e = started_at.elapsed();
            format!("[{}.{:03}s] ", e.as_secs(), e.subsec_millis())
        };

        let log_err_preamble = || {
            println!(
                "{}",
                format!(
                    "{}{}{}",
                    "✘ ".red().bold(),
                    dur.red().bold().dimmed(),
                    &file.name.red().bold()
                )
            );
            println!("{}", style("--- STEPS ---").on_yellow().bold());
            log_step_runs(&file.steps, 0);
        };

        let output_doc = write_yaml_snapshots(&file.original_source, &file);

        match res {
            Ok(_) => {
                if output_doc.trim() == file.original_source.trim() {
                    let msg = format!(
                        "{}{}{}",
                        "✓ ".green(),
                        dur.green().dimmed(),
                        &file.name.green()
                    );
                    println!("{}", msg.green());
                    Ok(())
                } else {
                    println!(
                        "{}",
                        format!(
                            "{}{}{}",
                            "⚠ ".yellow().bold(),
                            dur.yellow().bold().dimmed(),
                            &file.name.yellow().bold()
                        )
                    );
                    if !universe.ctx.params.interactive {
                        println!("{}\n", "--- SNAPSHOT CHANGED ---".on_bright_yellow().bold());
                        println!("{}", diff_snapshots(&file.original_source, &output_doc));
                        println!(
                            "\n{}",
                            "--- END SNAPSHOT CHANGE ---".on_bright_yellow().bold()
                        );
                        println!(
                            "\n{}",
                            "Run in interactive mode (-i) to accept new snapshots\n"
                                .bright_red()
                                .bold()
                        );
                    }
                    Err(HoldingError::SnapFailure { out: output_doc })
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

                    let matches = closest_strings(&comparator, comparisons);

                    eprintln!(
                        "Unable to resolve: \"{}\"\n{step_type} \"{}\" was not found.",
                        original_segment_string.red(),
                        comparator.yellow(),
                    );

                    matches
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, (s, score))| {
                            if i > 5 && score < 0.6 {
                                None
                            } else if i > 0 && score < 0.4 {
                                None
                            } else {
                                Some(parse_segments(&s).unwrap())
                            }
                        })
                        .collect::<Vec<_>>()
                };

                match &e.err {
                    HumaneStepError::External(ex) => match ex {
                        errors::HumaneInputError::NonexistentStep => {
                            log_err_preamble();
                            println!("{}", "--- ERROR ---".on_yellow().bold());
                            match &e.step {
                                HumaneTestStep::Ref {
                                    other_file,
                                    orig,
                                    hydrated_steps,
                                    state,
                                } => println!("{}", &e.red()),
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

                                    let matches = closest
                                        .into_iter()
                                        .map(|m| {
                                            let (actual_segments, _) = universe
                                                .instructions
                                                .get_key_value(&m)
                                                .expect("should exist in the global set");
                                            format!(
                                                "• {}",
                                                style(actual_segments.get_as_string()).cyan()
                                            )
                                        })
                                        .collect::<Vec<_>>();

                                    if matches.is_empty() {
                                        eprintln!("{}", "No similar instructions found".red());
                                    } else {
                                        eprintln!("Closest instructions:\n{}", matches.join("\n"));
                                    }
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

                                        let matches = closest
                                            .into_iter()
                                            .map(|m| {
                                                let (actual_segments, _) = universe
                                                    .retrievers
                                                    .get_key_value(&m)
                                                    .expect("should exist in the global set");
                                                format!(
                                                    "• {}",
                                                    style(actual_segments.get_as_string()).cyan()
                                                )
                                            })
                                            .collect::<Vec<_>>();

                                        if matches.is_empty() {
                                            eprintln!("{}", "No similar retrievals found".red());
                                        } else {
                                            eprintln!(
                                                "Closest retrievals:\n{}",
                                                matches.join("\n")
                                            );
                                        }
                                    } else {
                                        let closest = log_closest(
                                            "Assertion",
                                            &orig,
                                            &assertion,
                                            &universe.assertion_comparisons,
                                        );

                                        let matches = closest
                                            .into_iter()
                                            .map(|m| {
                                                let (actual_segments, _) = universe
                                                    .assertions
                                                    .get_key_value(&m)
                                                    .expect("should exist in the global set");
                                                format!(
                                                    "• {}",
                                                    style(actual_segments.get_as_string()).cyan()
                                                )
                                            })
                                            .collect::<Vec<_>>();

                                        if matches.is_empty() {
                                            eprintln!("{}", "No similar assertions found".red());
                                        } else {
                                            eprintln!(
                                                "Closest assertions:\n{}",
                                                matches.join("\n")
                                            );
                                        }
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
                Err(HoldingError::TestFailure)
            }
        }
    };

    let semaphore = Arc::new(tokio::sync::Semaphore::new(universe.ctx.params.concurrency));

    let mut hands = vec![];

    println!("\n{}\n", "Running tests".bold());

    match run_mode {
        RunMode::All => {
            for mut test in universe
                .tests
                .values()
                .filter(|v| v.r#type == HumaneFileType::Test)
                .cloned()
            {
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let uni = Arc::clone(&universe);
                hands.push(tokio::spawn(async move {
                    let start = Instant::now();
                    let res = run_humane_experiment(&mut test, Arc::clone(&uni)).await;
                    let holding_err = handle_res(uni, (&test, res), start);

                    drop(permit);

                    holding_err.map_err(|e| (test, e))
                }));
            }
        }
        RunMode::One(t) => {
            let mut test = universe.tests.get(&t).cloned().unwrap();
            let uni = Arc::clone(&universe);
            hands.push(tokio::spawn(async move {
                let start = Instant::now();
                let res = run_humane_experiment(&mut test, Arc::clone(&uni)).await;
                let holding_err = handle_res(uni, (&test, res), start);

                holding_err.map_err(|e| (test, e))
            }));
        }
    }

    let results = join_all(hands)
        .await
        .into_iter()
        .map(|outer_err| match outer_err {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(e) => panic!("Failed to await all tests: {e}"),
        })
        .collect::<Vec<_>>();

    let snapshot_failures = results
        .iter()
        .filter_map(|r| match r {
            Err((f, HoldingError::SnapFailure { out })) => Some((f, out)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut resolved_errors = 0;

    println!("\n{}\n", "Finished running tests".bold());

    let interactive = universe.ctx.params.interactive;
    if interactive && !snapshot_failures.is_empty() {
        let review_snapshots = match question(format!(
            "{} {}. Review now?",
            snapshot_failures.len(),
            if snapshot_failures.len() == 1 {
                "snapshot has changed"
            } else {
                "snapshots have changed"
            },
        )) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };

        if review_snapshots {
            let term = Term::stdout();

            for (file, failure) in results.iter().filter_map(|r| match r {
                Ok(_) => None,
                Err(e) => Some(e),
            }) {
                match failure {
                    HoldingError::TestFailure => {}
                    HoldingError::SnapFailure { out } => {
                        if confirm_snapshot(&term, &file, &out).is_ok_and(|v| v) {
                            resolved_errors += 1;

                            if let Err(e) = tokio::fs::write(&file.file_path, out).await {
                                eprintln!("Unable to write updates snapshot to disk.\n{e}");
                                std::process::exit(1);
                            }
                        }
                    }
                }
            }
            println!("\n\n");
        }
    }

    let duration = start.elapsed();
    let duration = if universe.ctx.params.porcelain {
        "".to_string()
    } else {
        format!(
            " in {}.{:03} seconds",
            duration.as_secs(),
            duration.subsec_millis()
        )
    };

    let failing = results.iter().filter(|r| r.is_err()).count() - resolved_errors;
    let passing = results.iter().filter(|r| r.is_ok()).count() + resolved_errors;

    println!(
        "{}\n{}",
        style(&format!("Passing tests: {}", passing)).cyan(),
        style(&format!("Failing tests: {}", failing)).cyan()
    );

    if failing > 0 {
        println!(
            "{}",
            style(&format!("\nSome tests failed{}", duration)).red()
        );
        std::process::exit(1);
    } else {
        println!(
            "{}",
            style(&format!("\nAll tests passed{}", duration)).green()
        );
    }
}
