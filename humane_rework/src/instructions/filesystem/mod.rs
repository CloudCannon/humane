use std::collections::HashMap;

use anyhow::{bail, Context};

use crate::civilization::Civilization;

use super::{HumaneInstruction, InstructionArgs};

mod new_file {
    use super::*;

    pub struct NewFile;

    inventory::submit! {
        &NewFile as &dyn HumaneInstruction
    }

    impl HumaneInstruction for NewFile {
        fn instruction(&self) -> &'static str {
            "I have a {filename} file with the content {contents}"
        }

        fn run(&self, args: &InstructionArgs, civ: &mut Civilization) -> Result<(), anyhow::Error> {
            let filename = args.get_str("filename")?;
            if filename.is_empty() {
                bail!("provided filename is empty");
            }

            let contents = args.get_str("contents")?;

            civ.write_file(filename, contents);

            Ok(())
        }
    }
}
