use std::collections::HashMap;

use async_trait::async_trait;

use crate::civilization::Civilization;
use crate::errors::{HumaneInputError, HumaneInternalError, HumaneStepError};

use super::{HumaneAssertion, HumaneInstruction, HumaneRetriever, SegmentArgs};

fn value_contains_value(
    base: &serde_json::Value,
    expected: &serde_json::Value,
) -> Result<bool, HumaneStepError> {
    use serde_json::Value::*;

    if base == expected {
        return Ok(true);
    };

    match (&base, &expected) {
        (Null, _) => Ok(false),
        (Bool(_), _) => Ok(false),
        (Number(_), _) => Ok(false),
        (String(s), Bool(b)) => {
            if s.contains(&b.to_string()) {
                Ok(true)
            } else {
                Ok(false)
            }
        },
        (String(s), Number(n)) => {
            if s.contains(&n.to_string()) {
                Ok(true)
            } else {
                Ok(false)
            }
        },
        (String(s), String(s2)) => {
            if s.contains(s2) {
                Ok(true)
            } else {
                Ok(false)
            }
        },
        // (Array(_), Null) => todo!(),
        // (Array(_), Bool(_)) => todo!(),
        // (Array(_), Number(_)) => todo!(),
        // (Array(_), String(_)) => todo!(),
        // (Array(_), Array(_)) => todo!(),
        // (Array(_), Object(_)) => todo!(),
        // (Object(_), Null) => todo!(),
        // (Object(_), Bool(_)) => todo!(),
        // (Object(_), Number(_)) => todo!(),
        // (Object(_), String(_)) => todo!(),
        // (Object(_), Array(_)) => todo!(),
        // (Object(_), Object(_)) => todo!(),
        _ => {
            Err(HumaneStepError::Internal(HumaneInternalError::Custom { msg: format!(
                "A comparison for these values has not been implemented.\n---\n{}\n---\ncannot compare with\n---\n{}\n---",
                serde_json::to_string(&base).expect("should be yaml-able"),
                serde_json::to_string(&expected).expect("should be yaml-able")
            ) }))
        }
    }
}

mod contain {
    use crate::errors::{HumaneInternalError, HumaneTestFailure};

    use super::*;

    pub struct Contain;

    inventory::submit! {
        &Contain as &dyn HumaneAssertion
    }

    #[async_trait]
    impl HumaneAssertion for Contain {
        fn segments(&self) -> &'static str {
            "contain {expected}"
        }

        async fn run(
            &self,
            base_value: serde_json::Value,
            args: &SegmentArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<(), HumaneStepError> {
            let expected = args.get_value("expected")?;

            if value_contains_value(&base_value, &expected)? {
                Ok(())
            } else {
                Err(HumaneStepError::Assertion(HumaneTestFailure::Custom {
                    msg: format!(
                        "The value\n---\n{}\n---\ndoes not contain\n---\n{}\n---",
                        serde_json::to_string(&base_value).expect("should be yaml-able"),
                        serde_json::to_string(&expected).expect("should be yaml-able")
                    ),
                }))
            }
        }
    }

    pub struct NotContain;

    inventory::submit! {
        &NotContain as &dyn HumaneAssertion
    }

    #[async_trait]
    impl HumaneAssertion for NotContain {
        fn segments(&self) -> &'static str {
            "not contain {expected}"
        }

        async fn run(
            &self,
            base_value: serde_json::Value,
            args: &SegmentArgs<'_>,
            civ: &mut Civilization,
        ) -> Result<(), HumaneStepError> {
            let expected = args.get_value("expected")?;

            if value_contains_value(&base_value, &expected)? {
                Err(HumaneStepError::Assertion(HumaneTestFailure::Custom {
                    msg: format!(
                        "The value\n---\n{}\n---\nshould not contain the following value, but does\n---\n{}\n---",
                        serde_json::to_string(&base_value).expect("should be yaml-able"),
                        serde_json::to_string(&expected).expect("should be yaml-able")
                    ),
                }))
            } else {
                Ok(())
            }
        }
    }
}
