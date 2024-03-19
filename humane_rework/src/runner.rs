use std::{collections::HashMap, path::PathBuf};

use crate::{
    instructions::{HumaneInstruction, HumaneSegments},
    HumaneTestFile,
};

pub async fn run_humane_experiment(
    input: &HumaneTestFile,
    all_files: &HashMap<PathBuf, HumaneTestFile>,
    instructions: &HashMap<HumaneSegments, &dyn HumaneInstruction>,
) {
    for setup_step in input.setup.iter() {
        match setup_step {
            crate::HumaneTestStep::Ref { other_file } => {
                println!("TODO: Need to load {other_file:?}")
            }
            crate::HumaneTestStep::Step { step, args } => {
                let Some(instruction) = instructions.get(step) else {
                    println!("Couldn't find instruction for {step:?}");
                    continue;
                };

                println!("Found an instruction!: {}", instruction.instruction());
            }
            crate::HumaneTestStep::Snapshot {
                snapshot,
                snapshot_content,
                args,
            } => {
                println!("TODO: Need to snapshot: {snapshot:?}");
            }
        }
    }

    for step in input.steps.iter() {
        match step {
            crate::HumaneTestStep::Ref { other_file } => {
                println!("TODO: Need to load {other_file:?}")
            }
            crate::HumaneTestStep::Step { step, args } => {
                let Some(instruction) = instructions.get(step) else {
                    println!("Couldn't find instruction for {step:?}");
                    continue;
                };

                println!("Found an instruction!: {}", instruction.instruction());
            }
            crate::HumaneTestStep::Snapshot {
                snapshot,
                snapshot_content,
                args,
            } => {
                println!("TODO: Need to snapshot: {snapshot:?}");
            }
        }
    }
}
