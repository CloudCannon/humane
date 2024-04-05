use std::{collections::HashMap, path::PathBuf, sync::Arc};

use console::style;

use crate::{
    civilization::Civilization,
    errors::{HumaneInputError, HumaneStepError, HumaneTestError},
    instructions::{HumaneInstruction, HumaneSegments, InstructionArgs},
    universe::Universe,
    HumaneTestFile, HumaneTestStep,
};

pub async fn run_humane_experiment(
    input: &HumaneTestFile,
    universe: Arc<Universe<'_>>,
) -> Result<Vec<String>, (Vec<String>, HumaneTestError)> {
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

    let mut step_logs = vec![];

    run_humane_steps(&input.setup, &mut civ, &mut step_logs)
        .await
        .map_err(|e| (step_logs.clone(), e))?;

    run_humane_steps(&input.steps, &mut civ, &mut step_logs)
        .await
        .map_err(|e| (step_logs.clone(), e))?;

    civ.shutdown().await;

    Ok(step_logs)
}

async fn run_humane_steps(
    steps: &Vec<HumaneTestStep>,
    civ: &mut Civilization<'_>,
    step_logs: &mut Vec<String>,
) -> Result<(), HumaneTestError> {
    for cur_step in steps.iter() {
        match cur_step {
            crate::HumaneTestStep::Ref {
                other_file,
                orig: _,
            } => {
                println!("TODO: Need to load {other_file:?}")
            }
            crate::HumaneTestStep::Step { step, args, orig } => {
                let Some((reference_segments, instruction)) =
                    civ.universe.instructions.get_key_value(step)
                else {
                    return Err(HumaneTestError {
                        err: HumaneStepError::External(HumaneInputError::NonexistentStep),
                        step: cur_step.clone(),
                        arg_str: cur_step.args_pretty(),
                    });
                };

                let instruction_args =
                    InstructionArgs::build(reference_segments, step, args, Some(&civ.universe.ctx))
                        .map_err(|e| HumaneTestError {
                            err: e.into(),
                            step: cur_step.clone(),
                            arg_str: cur_step.args_pretty(),
                        })?;

                instruction
                    .run(&instruction_args, civ)
                    .await
                    .map_err(|e| HumaneTestError {
                        err: e.into(),
                        step: cur_step.clone(),
                        arg_str: cur_step.args_pretty(),
                    })?;

                step_logs.push(format!("• {}", style(orig).green()));
            }
            crate::HumaneTestStep::Snapshot {
                snapshot,
                snapshot_content,
                args,
                orig: _,
            } => {
                println!("TODO: Need to snapshot: {snapshot:?}");
            }
        }
    }

    Ok(())
}
