use std::{collections::HashMap, hash::Hash};

use anyhow::Context;

use crate::{civilization::Civilization, parser::parse_instruction};

mod filesystem;

#[derive(Debug, Clone, PartialEq)]
pub enum HumaneSegment {
    Literal(String),
    Value(String),
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

pub trait HumaneInstruction: Sync {
    fn instruction(&self) -> &'static str;
    fn run(&self, args: &InstructionArgs, civ: &mut Civilization) -> Result<(), anyhow::Error>;
}

inventory::collect!(&'static dyn HumaneInstruction);

pub struct InstructionArgs {
    args: HashMap<String, serde_json::Value>,
}

impl InstructionArgs {
    fn get_str(&self, k: impl AsRef<str>) -> Result<&str, anyhow::Error> {
        Ok(self
            .args
            .get(k.as_ref())
            .context("argument not provided")?
            .as_str()
            .context("provided argument was not a string")?)
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

        impl HumaneInstruction for TestInstruction {
            fn instruction(&self) -> &'static str {
                "I am an instruction asking for {argument}"
            }

            fn run(
                &self,
                args: &InstructionArgs,
                civ: &mut Civilization,
            ) -> Result<(), anyhow::Error> {
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
