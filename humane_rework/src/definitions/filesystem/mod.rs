use std::collections::HashMap;

use async_trait::async_trait;

use crate::civilization::Civilization;
use crate::errors::{HumaneInputError, HumaneStepError};

use super::{HumaneInstruction, HumaneRetriever, SegmentArgs};

mod new_file {

    use super::*;

    pub struct NewFile;

    inventory::submit! {
        &NewFile as &dyn HumaneInstruction
    }

    #[async_trait]
    impl HumaneInstruction for NewFile {
        fn segments(&self) -> &'static str {
            "I have a {filename} file with the content {contents}"
        }

        async fn run(
            &self,
            args: &SegmentArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<(), HumaneStepError> {
            let filename = args.get_string("filename")?;
            if filename.is_empty() {
                return Err(HumaneInputError::ArgumentRequiresValue {
                    arg: "filename".to_string(),
                }
                .into());
            }

            let contents = args.get_string("contents")?;

            civ.write_file(&filename, &contents);

            Ok(())
        }
    }
}

mod read_files {
    use crate::errors::HumaneTestFailure;

    use super::*;

    pub struct PlainFile;

    inventory::submit! {
        &PlainFile as &dyn HumaneRetriever
    }

    #[async_trait]
    impl HumaneRetriever for PlainFile {
        fn segments(&self) -> &'static str {
            "The file {filename}"
        }

        async fn run(
            &self,
            args: &SegmentArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<serde_json::Value, HumaneStepError> {
            let filename = args.get_string("filename")?;

            if filename.is_empty() {
                return Err(HumaneInputError::ArgumentRequiresValue {
                    arg: "filename".to_string(),
                }
                .into());
            }

            let contents = civ.read_file(&filename)?;

            Ok(serde_json::Value::String(contents))
        }
    }
}
