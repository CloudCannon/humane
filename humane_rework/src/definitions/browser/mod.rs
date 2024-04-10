use std::collections::HashMap;

use async_trait::async_trait;

use crate::civilization::Civilization;
use crate::errors::{HumaneInputError, HumaneStepError};

use super::{HumaneInstruction, HumaneRetriever, SegmentArgs};

use pagebrowse_lib::{PagebrowseBuilder, Pagebrowser, PagebrowserWindow};

mod load_page {
    use super::*;

    pub struct LoadPage;

    inventory::submit! {
        &LoadPage as &dyn HumaneInstruction
    }

    #[async_trait]
    impl HumaneInstruction for LoadPage {
        fn segments(&self) -> &'static str {
            "In my browser, I load {url}"
        }

        async fn run(
            &self,
            args: &SegmentArgs<'_>,
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

    use futures::TryFutureExt;
    use tokio::time::sleep;

    use crate::errors::{HumaneInternalError, HumaneTestFailure};

    use super::*;

    fn harnessed(js: String) -> String {
        format!(
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
                    inner_response = await inner();
                }} catch (e) {{
                    humane_errs.push(`JavaScript error: ${{e}}`);
                }}
                
                return {{ humane_errs, inner_response }};
            "#
        )
    }

    async fn eval_and_return_js(
        js: String,
        civ: &mut Civilization<'_>,
    ) -> Result<serde_json::Value, HumaneStepError> {
        let Some(window) = civ.window.as_ref() else {
            return Err(HumaneStepError::External(
                HumaneInputError::StepRequirementsNotMet {
                    reason: "no page has been loaded into the browser for this test".into(),
                },
            ));
        };

        let value = window
            .evaluate_script(harnessed(js))
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

        Ok(map
            .get("inner_response")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }

    pub struct EvalJs;

    inventory::submit! {
        &EvalJs as &dyn HumaneInstruction
    }

    #[async_trait]
    impl HumaneInstruction for EvalJs {
        fn segments(&self) -> &'static str {
            "In my browser, I evaluate {js}"
        }

        async fn run(
            &self,
            args: &SegmentArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<(), HumaneStepError> {
            let js = args.get_string("js")?;

            _ = eval_and_return_js(js, civ).await?;

            Ok(())
        }
    }

    pub struct GetJs;

    inventory::submit! {
        &GetJs as &dyn HumaneRetriever
    }

    #[async_trait]
    impl HumaneRetriever for GetJs {
        fn segments(&self) -> &'static str {
            "In my browser, the result of {js}"
        }

        async fn run(
            &self,
            args: &SegmentArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<serde_json::Value, HumaneStepError> {
            let js = args.get_string("js")?;

            eval_and_return_js(js, civ).await
        }
    }
}
