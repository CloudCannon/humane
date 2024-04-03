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
    universe: &Universe<'_>,
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

    run_humane_steps(&input.setup, &mut civ).await?;

    run_humane_steps(&input.steps, &mut civ).await?;

    civ.shutdown().await;

    Ok(())
}

async fn run_humane_steps(
    steps: &Vec<HumaneTestStep>,
    civ: &mut Civilization<'_>,
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
                    });
                };

                let instruction_args = InstructionArgs::build(reference_segments, step, args)
                    .map_err(|e| HumaneTestError {
                        err: e.into(),
                        step: cur_step.clone(),
                    })?;

                instruction
                    .run(&instruction_args, civ)
                    .await
                    .map_err(|e| HumaneTestError {
                        err: e.into(),
                        step: cur_step.clone(),
                    })?;

                println!("• {}", style(orig).green());
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
