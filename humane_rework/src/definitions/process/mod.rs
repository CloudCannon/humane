use std::collections::HashMap;

use async_trait::async_trait;

use crate::civilization::Civilization;
use crate::errors::{HumaneInputError, HumaneStepError};

use super::{HumaneAssertion, HumaneInstruction, HumaneRetriever, SegmentArgs};

mod env_var {
    use super::*;

    pub struct EnvVar;

    inventory::submit! {
        &EnvVar as &dyn HumaneInstruction
    }

    #[async_trait]
    impl HumaneInstruction for EnvVar {
        fn segments(&self) -> &'static str {
            "I have the environment variable {name} set to {value}"
        }

        async fn run(
            &self,
            args: &SegmentArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<(), HumaneStepError> {
            let env_name = args.get_string("name")?;
            let env_value = args.get_string("value")?;

            civ.set_env(env_name.to_string(), env_value.to_string());

            Ok(())
        }
    }
}

mod run {
    use super::*;

    pub struct Run;

    inventory::submit! {
        &Run as &dyn HumaneInstruction
    }

    #[async_trait]
    impl HumaneInstruction for Run {
        fn segments(&self) -> &'static str {
            "I run {command}"
        }

        async fn run(
            &self,
            args: &SegmentArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<(), HumaneStepError> {
            let command = args.get_string("command")?;

            civ.run_command(command.to_string())?;

            Ok(())
        }
    }
}

mod stdio {
    use crate::errors::HumaneTestFailure;

    use super::*;

    pub struct StdOut;

    inventory::submit! {
        &StdOut as &dyn HumaneRetriever
    }

    #[async_trait]
    impl HumaneRetriever for StdOut {
        fn segments(&self) -> &'static str {
            "stdout"
        }

        async fn run(
            &self,
            args: &SegmentArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<serde_json::Value, HumaneStepError> {
            let Some(output) = &civ.last_command_output else {
                return Err(HumaneStepError::Assertion(HumaneTestFailure::Custom {
                    msg: "no stdout exists".into(),
                }));
            };

            Ok(output.stdout.clone().into())
        }
    }

    pub struct StdErr;

    inventory::submit! {
        &StdErr as &dyn HumaneRetriever
    }

    #[async_trait]
    impl HumaneRetriever for StdErr {
        fn segments(&self) -> &'static str {
            "stderr"
        }

        async fn run(
            &self,
            args: &SegmentArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<serde_json::Value, HumaneStepError> {
            let Some(output) = &civ.last_command_output else {
                return Err(HumaneStepError::Assertion(HumaneTestFailure::Custom {
                    msg: "no stderr exists".into(),
                }));
            };

            Ok(output.stderr.clone().into())
        }
    }
}
