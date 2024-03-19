use std::{collections::HashMap, path::PathBuf};

use anyhow::bail;
use serde_json::{Map, Value};

use crate::{
    instructions::{HumaneSegment, HumaneSegments},
    HumaneTestFile, HumaneTestStep,
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
    type Error = anyhow::Error;

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
    type Error = anyhow::Error;

    fn try_from(value: RawHumaneTestStep) -> Result<Self, Self::Error> {
        match value {
            RawHumaneTestStep::Ref { r#ref } => Ok(HumaneTestStep::Ref {
                other_file: PathBuf::try_from(r#ref)?,
            }),
            RawHumaneTestStep::BareStep(step) => Ok(HumaneTestStep::Step {
                step: parse_instruction(&step)?,
                args: HashMap::new(),
            }),
            RawHumaneTestStep::StepWithParams { step, other } => Ok(HumaneTestStep::Step {
                step: parse_instruction(&step)?,
                args: HashMap::from_iter(other.into_iter()),
            }),
            RawHumaneTestStep::Snapshot { snapshot, other } => Ok(HumaneTestStep::Snapshot {
                snapshot: parse_instruction(&snapshot)?,
                snapshot_content: None,
                args: HashMap::from_iter(other.into_iter()),
            }),
        }
    }
}

pub fn parse_file(s: &str) -> Result<HumaneTestFile, anyhow::Error> {
    let raw_test = serde_yaml::from_str::<RawHumaneTestFile>(s)?;

    raw_test.try_into()
}

pub fn parse_instruction(s: &str) -> Result<HumaneSegments, anyhow::Error> {
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
                        segments.push(Value("".to_string()));
                    } else {
                        segments.push(Value(s[inner_start..i].to_string()));
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
        InstMode::InQuote(_, q) => bail!("Quoted value was not closed, expected {q}"),
        InstMode::InCurly(_) => bail!("Variable was not closed, expected }}"),
    }

    Ok(HumaneSegments { segments })
}

#[cfg(test)]
mod test {
    use super::*;
    use HumaneSegment::*;

    #[test]
    fn test_parsing_instructions() {
        let instruction = parse_instruction("I run my program").expect("Valid instruction");
        // We test equality on the segments directly,
        // as the instruction itself uses a looser comparison that doesn't
        // look inside Value or Variable segments.
        assert_eq!(
            instruction.segments,
            vec![Literal("I run my program".to_string())]
        );

        let instruction = parse_instruction("I have a \"public/cat/'index'.html\" file with the body '<h1>Happy post about \"cats</h1>'").expect("Valid instruction");
        assert_eq!(
            instruction.segments,
            vec![
                Literal("I have a ".to_string()),
                Value("public/cat/'index'.html".to_string()),
                Literal(" file with the body ".to_string()),
                Value("<h1>Happy post about \"cats</h1>".to_string())
            ]
        );

        let instruction =
            parse_instruction("In my browser, ''I eval {j\"s} and 'x'").expect("Valid instruction");
        assert_eq!(
            instruction.segments,
            vec![
                Literal("In my browser, ".to_string()),
                Value("".to_string()),
                Literal("I eval ".to_string()),
                Variable("j\"s".to_string()),
                Literal(" and ".to_string()),
                Value("x".to_string()),
            ]
        );
    }
}
