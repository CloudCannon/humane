use async_recursion::async_recursion;
use futures::FutureExt;
use normalize_path::NormalizePath;
use similar_string::find_best_similarity;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use console::style;

use crate::{
    civilization::Civilization,
    definitions::HumaneInstruction,
    errors::{HumaneInputError, HumaneStepError, HumaneTestError},
    segments::SegmentArgs,
    universe::Universe,
    HumaneTestFile, HumaneTestStep, HumaneTestStepState,
};

pub async fn run_humane_experiment(
    input: &mut HumaneTestFile,
    universe: Arc<Universe<'_>>,
) -> Result<(), HumaneTestError> {
    let mut civ = Civilization {
        tmp_dir: None,
        last_command_output: None,
        assigned_server_port: None,
        window: None,
        threads: vec![],
        handles: vec![],
        env_vars: HashMap::new(),
        universe,
    };

    run_humane_steps(&input.file_directory, &mut input.steps, &mut civ).await?;

    civ.shutdown().await;

    Ok(())
}

#[async_recursion]
async fn run_humane_steps(
    file_directory: &String,
    steps: &mut Vec<HumaneTestStep>,
    civ: &mut Civilization<'_>,
) -> Result<(), HumaneTestError> {
    for cur_step in steps.iter_mut() {
        let marked_base_step = cur_step.clone();
        let marked_base_args = cur_step.args_pretty();

        let mark_and_return_step_error = |e: HumaneStepError, state: &mut HumaneTestStepState| {
            *state = HumaneTestStepState::Failed;
            HumaneTestError {
                err: e.into(),
                step: marked_base_step.clone(),
                arg_str: marked_base_args.clone(),
            }
        };

        match cur_step {
            crate::HumaneTestStep::Ref {
                other_file,
                orig: _,
                hydrated_steps,
                state,
            } => {
                let target_path = PathBuf::from(file_directory)
                    .join(other_file)
                    .normalize()
                    .to_string_lossy()
                    .into_owned();
                let Some(target_file) = civ.universe.tests.get(&target_path).cloned() else {
                    let avail = civ.universe.tests.keys().collect::<Vec<_>>();
                    let closest = find_best_similarity(&target_path, &avail).map(|s| s.0);
                    return Err(mark_and_return_step_error(
                        HumaneStepError::External(HumaneInputError::InvalidRef {
                            input: target_path,
                            closest: closest.unwrap_or_else(|| "<nothing found>".to_string()),
                        }),
                        state,
                    ));
                };

                *hydrated_steps = Some(target_file.steps);

                match run_humane_steps(
                    &target_file.file_directory,
                    hydrated_steps.as_mut().unwrap(),
                    civ,
                )
                .await
                {
                    Ok(_) => {
                        *state = HumaneTestStepState::Passed;
                    }
                    Err(e) => {
                        *state = HumaneTestStepState::Failed;
                        return Err(e);
                    }
                }
            }
            crate::HumaneTestStep::Instruction {
                step,
                args,
                orig,
                state,
            } => {
                let Some((reference_segments, instruction)) =
                    civ.universe.instructions.get_key_value(step)
                else {
                    *state = HumaneTestStepState::Failed;
                    return Err(mark_and_return_step_error(
                        HumaneStepError::External(HumaneInputError::NonexistentStep),
                        state,
                    ));
                };

                let instruction_args =
                    SegmentArgs::build(reference_segments, step, args, Some(&civ.universe.ctx))
                        .map_err(|e| mark_and_return_step_error(e.into(), state))?;

                instruction
                    .run(&instruction_args, civ)
                    .await
                    .map_err(|e| mark_and_return_step_error(e.into(), state))?;

                *state = HumaneTestStepState::Passed;
            }
            crate::HumaneTestStep::Assertion {
                retrieval,
                assertion,
                args,
                orig,
                state,
            } => {
                let Some((reference_ret, retrieval_step)) =
                    civ.universe.retrievers.get_key_value(retrieval)
                else {
                    return Err(mark_and_return_step_error(
                        HumaneStepError::External(HumaneInputError::NonexistentStep),
                        state,
                    ));
                };

                let retrieval_args =
                    SegmentArgs::build(reference_ret, retrieval, args, Some(&civ.universe.ctx))
                        .map_err(|e| mark_and_return_step_error(e.into(), state))?;

                let value = retrieval_step
                    .run(&retrieval_args, civ)
                    .await
                    .map_err(|e| mark_and_return_step_error(e.into(), state))?;

                let Some((reference_assert, assertion_step)) =
                    civ.universe.assertions.get_key_value(assertion)
                else {
                    return Err(mark_and_return_step_error(
                        HumaneStepError::External(HumaneInputError::NonexistentStep),
                        state,
                    ));
                };

                let assertion_args =
                    SegmentArgs::build(reference_assert, assertion, args, Some(&civ.universe.ctx))
                        .map_err(|e| mark_and_return_step_error(e.into(), state))?;

                assertion_step
                    .run(value, &assertion_args, civ)
                    .await
                    .map_err(|e| mark_and_return_step_error(e.into(), state))?;

                *state = HumaneTestStepState::Passed;
            }
            crate::HumaneTestStep::Snapshot {
                snapshot,
                snapshot_content,
                args,
                orig: _,
                state,
            } => {
                let Some((reference_ret, retrieval_step)) =
                    civ.universe.retrievers.get_key_value(snapshot)
                else {
                    return Err(mark_and_return_step_error(
                        HumaneStepError::External(HumaneInputError::NonexistentStep),
                        state,
                    ));
                };

                let retrieval_args =
                    SegmentArgs::build(reference_ret, snapshot, args, Some(&civ.universe.ctx))
                        .map_err(|e| mark_and_return_step_error(e.into(), state))?;

                let value = retrieval_step
                    .run(&retrieval_args, civ)
                    .await
                    .map_err(|e| mark_and_return_step_error(e.into(), state))?;

                let value_content = match &value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => serde_yaml::to_string(&value).expect("snapshot value is serializable"),
                };

                *snapshot_content = Some(value_content);
                *state = HumaneTestStepState::Passed;
            }
        }
    }

    Ok(())
}
