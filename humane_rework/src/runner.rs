use std::{collections::HashMap, path::PathBuf};

use crate::{
    civilization::Civilization,
    errors::{HumaneInputError, HumaneStepError, HumaneTestError},
    instructions::{HumaneInstruction, HumaneSegments, InstructionArgs},
    HumaneTestFile, HumaneTestStep,
};

pub async fn run_humane_experiment(
    input: &HumaneTestFile,
    all_files: &HashMap<PathBuf, HumaneTestFile>,
    instructions: &HashMap<HumaneSegments, &dyn HumaneInstruction>,
) -> Result<(), HumaneTestError> {
    let mut civ = Civilization {
        tmp_dir: None,
        last_command_output: None,
        assigned_server_port: None,
        threads: vec![],
        handles: vec![],
        env_vars: HashMap::new(),
    };

    run_humane_steps(&input.setup, all_files, instructions, &mut civ).await?;

    run_humane_steps(&input.steps, all_files, instructions, &mut civ).await?;

    Ok(())
}

async fn run_humane_steps(
    steps: &Vec<HumaneTestStep>,
    all_files: &HashMap<PathBuf, HumaneTestFile>,
    instructions: &HashMap<HumaneSegments, &dyn HumaneInstruction>,
    civ: &mut Civilization,
) -> Result<(), HumaneTestError> {
    for cur_step in steps.iter() {
        match cur_step {
            crate::HumaneTestStep::Ref {
                other_file,
                orig: _,
            } => {
                println!("TODO: Need to load {other_file:?}")
            }
            crate::HumaneTestStep::Step {
                step,
                args,
                orig: _,
            } => {
                let Some((reference_segments, instruction)) = instructions.get_key_value(step)
                else {
                    println!("Couldn't find instruction for {step:?}");
                    continue;
                };

                println!("Found an instruction!: {}", instruction.instruction());

                let instruction_args = InstructionArgs::build(reference_segments, step, args)
                    .map_err(|e| HumaneTestError {
                        err: e.into(),
                        step: cur_step.clone(),
                    })?;

                instruction
                    .run(&instruction_args, civ)
                    .map_err(|e| HumaneTestError {
                        err: e.into(),
                        step: cur_step.clone(),
                    })?;
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
