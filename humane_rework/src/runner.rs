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

    run_humane_steps(&mut input.steps, &mut civ).await?;

    civ.shutdown().await;

    Ok(())
}

async fn run_humane_steps(
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
                state,
            } => {
                println!("TODO: Need to load {other_file:?}")
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
                println!("TODO: Need to snapshot: {snapshot:?}");
            }
        }
    }

    Ok(())
}
