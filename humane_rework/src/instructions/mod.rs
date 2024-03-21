use std::{collections::HashMap, hash::Hash};

use async_trait::async_trait;

use crate::{
    civilization::Civilization,
    errors::{HumaneInputError, HumaneStepError},
    parser::parse_instruction,
};

mod filesystem;
mod hosting;

#[derive(Debug, Clone, PartialEq)]
pub enum HumaneSegment {
    Literal(String),
    Value(serde_json::Value),
    Variable(String),
}

#[derive(Debug, Clone)]
pub struct HumaneSegments {
    pub segments: Vec<HumaneSegment>,
}

impl Hash for HumaneSegments {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use HumaneSegment::*;

        for seg in &self.segments {
            match seg {
                Literal(lit) => lit.hash(state),
                Value(_) | Variable(_) => 0.hash(state),
            }
        }
    }
}

impl PartialEq for HumaneSegments {
    fn eq(&self, other: &Self) -> bool {
        use HumaneSegment::*;

        if self.segments.len() != other.segments.len() {
            return false;
        }

        self.segments
            .iter()
            .zip(other.segments.iter())
            .all(|(a, b)| match a {
                Literal(_) => a == b,
                Value(_) | Variable(_) => matches!(b, Variable(_)),
            })
    }
}

impl Eq for HumaneSegments {}

impl HumaneSegments {
    pub fn get_comparison_string(&self) -> String {
        use HumaneSegment::*;

        self.segments
            .iter()
            .map(|s| match s {
                Literal(l) => l,
                Value(_) | Variable(_) => "{___}",
            })
            .collect()
    }

    pub fn get_as_string(&self) -> String {
        use HumaneSegment::*;

        self.segments
            .iter()
            .map(|s| match s {
                Literal(l) => l.clone(),
                Value(val) => format!("\"{val}\""),
                Variable(var) => format!("{{{var}}}"),
            })
            .collect()
    }
}

#[async_trait]
pub trait HumaneInstruction: Sync {
    fn instruction(&self) -> &'static str;
    async fn run(
        &self,
        args: &InstructionArgs<'_>,
        civ: &mut Civilization,
    ) -> Result<(), HumaneStepError>;
}

inventory::collect!(&'static dyn HumaneInstruction);

pub struct InstructionArgs<'a> {
    args: HashMap<String, &'a serde_json::Value>,
}

impl<'a> InstructionArgs<'a> {
    pub fn build(
        reference_instruction: &HumaneSegments,
        supplied_instruction: &'a HumaneSegments,
        supplied_args: &'a HashMap<String, serde_json::Value>,
    ) -> Result<InstructionArgs<'a>, HumaneInputError> {
        let mut args = HashMap::new();

        for (reference, supplied) in reference_instruction
            .segments
            .iter()
            .zip(supplied_instruction.segments.iter())
        {
            let HumaneSegment::Variable(inst_key) = reference else {
                continue;
            };

            match supplied {
                HumaneSegment::Value(val) => {
                    args.insert(inst_key.to_owned(), val);
                }
                HumaneSegment::Variable(var) => {
                    let Some(var_val) = supplied_args.get(var) else {
                        return Err(HumaneInputError::NonexistentArgument {
                            arg: var.to_string(),
                            has: supplied_args.keys().cloned().collect::<Vec<_>>().join(", "),
                        });
                    };
                    args.insert(inst_key.to_owned(), var_val);
                }
                HumaneSegment::Literal(l) => panic!("{l} should be unreachable"),
            }
        }

        Ok(Self { args })
    }

    fn get_str(&self, k: impl AsRef<str>) -> Result<&str, HumaneInputError> {
        let Some(value) = self.args.get(k.as_ref()) else {
            return Err(HumaneInputError::NonexistentArgument {
                arg: k.as_ref().to_string(),
                has: self.args.keys().cloned().collect::<Vec<_>>().join(", "),
            });
        };

        let Some(str) = value.as_str() else {
            let found = match value {
                serde_json::Value::Null => "null",
                serde_json::Value::Bool(_) => "boolean",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::Array(_) => "array",
                serde_json::Value::Object(_) => "object",
                serde_json::Value::String(_) => unreachable!(),
            };
            return Err(HumaneInputError::IncorrectArgumentType {
                arg: k.as_ref().to_string(),
                was: found.to_string(),
                expected: "string".to_string(),
            });
        };

        Ok(str)
    }
}

pub fn register_instructions() -> HashMap<HumaneSegments, &'static dyn HumaneInstruction> {
    HashMap::<_, _>::from_iter(
        (inventory::iter::<&dyn HumaneInstruction>)
            .into_iter()
            .map(|i| {
                let segments = parse_instruction(i.instruction())
                    .expect("builtin instructions should be parseable");

                (segments, *i)
            }),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_building_args() {
        let instruction_def = parse_instruction("I have a {name} file with the contents {var}")
            .expect("Valid instruction");

        let user_instruction =
            parse_instruction("I have a \"index.html\" file with the contents ':)'")
                .expect("Valid instruction");

        let input = HashMap::new();

        let args = InstructionArgs::build(&instruction_def, &user_instruction, &input)
            .expect("Args built successfully");

        let Ok(str) = args.get_str("name") else {
            panic!("Argument was not a string, got {:?}", args.get_str("name"));
        };
        assert_eq!(str, "index.html");
    }

    // Instructions should alias to each other regardless of the contents of their
    // variables or values.
    #[test]
    fn test_instruction_equality() {
        let instruction_a = parse_instruction("I have a 'index.html' file with the contents {var}")
            .expect("Valid instruction");

        let instruction_b = parse_instruction("I have a {filename} file with the contents {var}")
            .expect("Valid instruction");

        let instruction_c = parse_instruction("I have one {filename} file with the contents {var}")
            .expect("Valid instruction");

        assert_eq!(instruction_a, instruction_b);

        let mut map = HashMap::new();
        map.insert(&instruction_b, "b");

        assert_eq!(map.get(&&instruction_a), Some(&"b"));

        assert_ne!(instruction_b, instruction_c);
        assert_eq!(map.get(&&instruction_c), None);
    }

    #[test]
    fn test_getting_an_instruction() {
        pub struct TestInstruction;

        inventory::submit! {
            &TestInstruction as &dyn HumaneInstruction
        }

        #[async_trait]
        impl HumaneInstruction for TestInstruction {
            fn instruction(&self) -> &'static str {
                "I am an instruction asking for {argument}"
            }

            async fn run(
                &self,
                args: &InstructionArgs<'_>,
                civ: &mut Civilization,
            ) -> Result<(), HumaneStepError> {
                Ok(())
            }
        }

        let users_instruction =
            parse_instruction("I am an instruction asking for \"this argument\"")
                .expect("Valid instruction");

        let all_instructions = register_instructions();
        let matching_instruction = all_instructions
            .get(&users_instruction)
            .expect("should be able to retrieve instruction");

        assert_eq!(
            matching_instruction.instruction(),
            "I am an instruction asking for {argument}"
        );
    }
}
