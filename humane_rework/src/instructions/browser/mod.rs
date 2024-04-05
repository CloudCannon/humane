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
                args.get_string("url")?
            );

            let window = civ.universe.pagebrowser.get_window().await.unwrap();

            window
                .navigate(url.to_string(), true)
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

    use crate::errors::{HumaneInternalError, HumaneTestFailure};

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
            let js = args.get_string("js")?;

            let Some(window) = civ.window.as_ref() else {
                return Err(HumaneStepError::External(
                    HumaneInputError::StepRequirementsNotMet {
                        reason: "no page has been loaded into the browser for this test".into(),
                    },
                ));
            };

            let harnessed = format!(
                r#"
                const humane_errs = [];

                const humane = {{
                    assert_eq: (left, right) => {{
                        if (left !== right) {{
                            humane_errs.push(`Equality Assertion failed. Left: ${{JSON.stringify(left)}}, Right: ${{JSON.stringify(right)}}`);
                        }}
                    }}
                }}
                
                const inner = async () => {{
                    {js}
                }}

                let inner_response;
                try {{
                    let inner_response = await inner();
                }} catch (e) {{
                    humane_errs.push(`JavaScript error: ${{e}}`);
                }}
                
                return {{ humane_errs, inner_response }};
            "#
            );

            let value = window
                .evaluate_script(harnessed)
                .await
                .map_err(|inner| HumaneStepError::Internal(inner.into()))?;

            let Some(serde_json::Value::Object(map)) = &value else {
                return Err(HumaneStepError::External(HumaneInputError::StepError {
                    reason: "JavaScript failed to parse and run".to_string(),
                }));
            };

            let Some(serde_json::Value::Array(errors)) = map.get("humane_errs") else {
                return Err(HumaneStepError::Internal(HumaneInternalError::Custom {
                    msg: format!("JavaScript returned an unexpected value: {value:?}"),
                }));
            };

            if !errors.is_empty() {
                return Err(HumaneStepError::Assertion(HumaneTestFailure::Custom {
                    msg: errors
                        .iter()
                        .map(|v| v.as_str().unwrap())
                        .collect::<Vec<_>>()
                        .join("\n"),
                }));
            }

            Ok(())
        }
    }
}
