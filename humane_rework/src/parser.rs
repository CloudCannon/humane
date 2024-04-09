use std::{collections::HashMap, path::PathBuf};

use serde_json::{Map, Value};

use crate::{
    errors::HumaneInputError,
    segments::{HumaneSegment, HumaneSegments},
    HumaneTestFile, HumaneTestStep, HumaneTestStepState,
};

#[derive(serde::Serialize, serde::Deserialize)]
struct RawHumaneTestFile {
    test: String,
    setup: Vec<RawHumaneTestStep>,
    steps: Vec<RawHumaneTestStep>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum RawHumaneTestStep {
    Ref {
        r#ref: String,
    },
    BareStep(String),
    StepWithParams {
        step: String,
        #[serde(flatten)]
        other: Map<String, Value>,
    },
    Snapshot {
        snapshot: String,
        #[serde(flatten)]
        other: Map<String, Value>,
    },
}

impl TryFrom<RawHumaneTestFile> for HumaneTestFile {
    type Error = HumaneInputError;

    fn try_from(value: RawHumaneTestFile) -> Result<Self, Self::Error> {
        let mut setup = Vec::with_capacity(value.setup.len());
        for setup_step in value.setup {
            setup.push(setup_step.try_into()?);
        }

        let mut steps = Vec::with_capacity(value.steps.len());
        for step in value.steps {
            steps.push(step.try_into()?);
        }

        Ok(HumaneTestFile {
            test: value.test,
            setup,
            steps,
        })
    }
}

impl TryFrom<RawHumaneTestStep> for HumaneTestStep {
    type Error = HumaneInputError;

    fn try_from(value: RawHumaneTestStep) -> Result<Self, Self::Error> {
        match value {
            RawHumaneTestStep::Ref { r#ref } => Ok(HumaneTestStep::Ref {
                other_file: PathBuf::try_from(&r#ref).map_err(|_| {
                    HumaneInputError::InvalidPath {
                        input: r#ref.clone(),
                    }
                })?,
                orig: r#ref,
                state: HumaneTestStepState::Dormant,
            }),
            RawHumaneTestStep::BareStep(step) => parse_step(step, HashMap::new()),
            RawHumaneTestStep::StepWithParams { step, other } => {
                parse_step(step, HashMap::from_iter(other.into_iter()))
            }
            RawHumaneTestStep::Snapshot { snapshot, other } => Ok(HumaneTestStep::Snapshot {
                snapshot: parse_segments(&snapshot)?,
                snapshot_content: None,
                args: HashMap::from_iter(other.into_iter()),
                orig: snapshot,
                state: HumaneTestStepState::Dormant,
            }),
        }
    }
}

fn parse_step(
    step: String,
    args: HashMap<String, Value>,
) -> Result<HumaneTestStep, HumaneInputError> {
    if let Some((retrieval, assertion)) = step.split_once(" should ") {
        Ok(HumaneTestStep::Assertion {
            retrieval: parse_segments(retrieval)?,
            assertion: parse_segments(assertion)?,
            args,
            orig: step,
            state: HumaneTestStepState::Dormant,
        })
    } else {
        Ok(HumaneTestStep::Instruction {
            step: parse_segments(&step)?,
            args,
            orig: step,
            state: HumaneTestStepState::Dormant,
        })
    }
}

pub fn parse_file(s: &str) -> Result<HumaneTestFile, HumaneInputError> {
    let raw_test = serde_yaml::from_str::<RawHumaneTestFile>(s)?;

    raw_test.try_into()
}

pub fn parse_segments(s: &str) -> Result<HumaneSegments, HumaneInputError> {
    let mut segments = vec![];
    use HumaneSegment::*;

    enum InstMode {
        None(usize),
        InQuote(usize, char),
        InCurly(usize),
    }

    let mut mode = InstMode::None(0);

    for (i, c) in s.chars().enumerate() {
        match &mut mode {
            InstMode::None(start) => match c {
                '"' => {
                    segments.push(Literal(s[*start..i].to_string()));
                    mode = InstMode::InQuote(i, '"');
                }
                '\'' => {
                    segments.push(Literal(s[*start..i].to_string()));
                    mode = InstMode::InQuote(i, '\'');
                }
                '{' => {
                    segments.push(Literal(s[*start..i].to_string()));
                    mode = InstMode::InCurly(i);
                }
                _ => {}
            },
            InstMode::InQuote(start, quote) => match c {
                c if c == *quote => {
                    let inner_start = *start + 1;
                    if i == inner_start {
                        segments.push(Value(serde_json::Value::String("".to_string())));
                    } else {
                        segments.push(Value(serde_json::Value::String(
                            s[inner_start..i].to_string(),
                        )));
                    }
                    mode = InstMode::None(i + 1);
                }
                _ => {}
            },
            InstMode::InCurly(start) => match c {
                '}' => {
                    let inner_start = *start + 1;
                    if i == inner_start {
                        segments.push(Variable("".to_string()));
                    } else {
                        segments.push(Variable(s[inner_start..i].to_string()));
                    }
                    mode = InstMode::None(i + 1);
                }
                _ => {}
            },
        }
    }

    match mode {
        InstMode::None(start) => {
            if start < s.len() {
                segments.push(Literal(s[start..].to_string()));
            }
        }
        InstMode::InQuote(_, q) => return Err(HumaneInputError::UnclosedValue { expected: q }),
        InstMode::InCurly(_) => return Err(HumaneInputError::UnclosedValue { expected: '}' }),
    }

    Ok(HumaneSegments { segments })
}

#[cfg(test)]
mod test {
    use super::*;
    use HumaneSegment::*;

    fn st(s: &str) -> serde_json::Value {
        serde_json::Value::String(s.to_string())
    }

    #[test]
    fn test_parsing_segments() {
        let segments = parse_segments("I run my program").expect("Valid segments");
        // We test equality on the segments directly,
        // as the segments itself uses a looser comparison that doesn't
        // look inside Value or Variable segments.
        assert_eq!(
            segments.segments,
            vec![Literal("I run my program".to_string())]
        );

        let segments = parse_segments("I have a \"public/cat/'index'.html\" file with the body '<h1>Happy post about \"cats</h1>'").expect("Valid segments");
        assert_eq!(
            segments.segments,
            vec![
                Literal("I have a ".to_string()),
                Value(st("public/cat/'index'.html")),
                Literal(" file with the body ".to_string()),
                Value(st("<h1>Happy post about \"cats</h1>"))
            ]
        );

        let segments =
            parse_segments("In my browser, ''I eval {j\"s} and 'x'").expect("Valid segments");
        assert_eq!(
            segments.segments,
            vec![
                Literal("In my browser, ".to_string()),
                Value(st("")),
                Literal("I eval ".to_string()),
                Variable("j\"s".to_string()),
                Literal(" and ".to_string()),
                Value(st("x")),
            ]
        );
    }

    #[test]
    fn test_parsing_steps() {
        let Ok(step) = parse_step("I have a {js} file".to_string(), HashMap::new()) else {
            panic!("Step did not parse");
        };

        assert_eq!(
            step,
            HumaneTestStep::Instruction {
                step: HumaneSegments {
                    segments: vec![
                        Literal("I have a ".to_string()),
                        Variable("js".to_string()),
                        Literal(" file".to_string())
                    ]
                },
                args: HashMap::new(),
                orig: "I have a {js} file".to_string(),
                state: HumaneTestStepState::Dormant
            }
        );

        let Ok(step) = parse_step(
            "The file {name} should contain {html}".to_string(),
            HashMap::new(),
        ) else {
            panic!("Step did not parse");
        };

        assert_eq!(
            step,
            HumaneTestStep::Assertion {
                retrieval: HumaneSegments {
                    segments: vec![
                        Literal("The file ".to_string()),
                        Variable("name".to_string())
                    ]
                },
                assertion: HumaneSegments {
                    segments: vec![
                        Literal("contain ".to_string()),
                        Variable("html".to_string()),
                    ]
                },
                args: HashMap::new(),
                orig: "The file {name} should contain {html}".to_string(),
                state: HumaneTestStepState::Dormant
            }
        );
    }
}
