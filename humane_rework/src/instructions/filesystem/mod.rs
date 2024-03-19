use std::collections::HashMap;

use crate::civilization::Civilization;

use super::{HumaneInstruction, InstructionArgs};

mod new_file {
    use crate::errors::{HumaneInputError, HumaneStepError};

    use super::*;

    pub struct NewFile;

    inventory::submit! {
        &NewFile as &dyn HumaneInstruction
    }

    impl HumaneInstruction for NewFile {
        fn instruction(&self) -> &'static str {
            "I have a {filename} file with the content {contents}"
        }

        fn run(
            &self,
            args: &InstructionArgs,
            civ: &mut Civilization,
        ) -> Result<(), HumaneStepError> {
            let filename = args.get_str("filename")?;
            if filename.is_empty() {
                return Err(HumaneInputError::ArgumentRequiresValue {
                    arg: "filename".to_string(),
                }
                .into());
            }

            let contents = args.get_str("contents")?;

            civ.write_file(filename, contents);

            Ok(())
        }
    }
}
