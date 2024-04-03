use std::collections::HashMap;

use async_trait::async_trait;

use crate::civilization::Civilization;
use crate::errors::{HumaneInputError, HumaneStepError};

use super::{HumaneInstruction, InstructionArgs};

use pagebrowse_lib::{PagebrowseBuilder, Pagebrowser, PagebrowserWindow};

mod load_page {
    use super::*;

    pub struct LoadPage;

    inventory::submit! {
        &LoadPage as &dyn HumaneInstruction
    }

    #[async_trait]
    impl HumaneInstruction for LoadPage {
        fn instruction(&self) -> &'static str {
            "In my browser, I load {url}"
        }

        async fn run(
            &self,
            args: &InstructionArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<(), HumaneStepError> {
            let url = format!(
                "http://localhost:{}{}",
                civ.ensure_port(),
                args.get_str("url")?
            );

            let window = civ.universe.pagebrowser.get_window().await.unwrap();

            window
                .navigate(url.to_string())
                .await
                .map_err(|inner| HumaneStepError::Internal(inner.into()))?;

            civ.window = Some(window);

            Ok(())
        }
    }
}

mod eval_js {
    use std::time::Duration;

    use tokio::time::sleep;

    use super::*;

    pub struct EvalJs;

    inventory::submit! {
        &EvalJs as &dyn HumaneInstruction
    }

    #[async_trait]
    impl HumaneInstruction for EvalJs {
        fn instruction(&self) -> &'static str {
            "In my browser, I evaluate {js}"
        }

        async fn run(
            &self,
            args: &InstructionArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<(), HumaneStepError> {
            let js = args.get_str("js")?;

            let Some(window) = civ.window.as_ref() else {
                return Err(HumaneStepError::External(
                    HumaneInputError::StepRequirementsNotMet {
                        reason: "no page has been loaded into the browser for this test".into(),
                    },
                ));
            };

            window
                .evaluate_script(js.to_string())
                .await
                .map_err(|inner| HumaneStepError::Internal(inner.into()))?;

            // sleep(Duration::from_secs(20)).await;

            Ok(())
        }
    }
}
