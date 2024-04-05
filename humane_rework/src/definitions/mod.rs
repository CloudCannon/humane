use std::{collections::HashMap, hash::Hash};

use async_trait::async_trait;

use crate::{
    civilization::Civilization,
    errors::{HumaneInputError, HumaneStepError},
    options::{HumaneContext, HumaneParams},
    parser::parse_segments,
    segments::{HumaneSegment, HumaneSegments, SegmentArgs},
};

mod assertions;
mod browser;
mod filesystem;
mod hosting;
mod process;

/// Main instructions, generally start with "I ..."
#[async_trait]
pub trait HumaneInstruction: Sync {
    fn segments(&self) -> &'static str;
    async fn run(
        &self,
        args: &SegmentArgs<'_>,
        civ: &mut Civilization,
    ) -> Result<(), HumaneStepError>;
}

inventory::collect!(&'static dyn HumaneInstruction);

pub fn register_instructions() -> HashMap<HumaneSegments, &'static dyn HumaneInstruction> {
    HashMap::<_, _>::from_iter(
        (inventory::iter::<&dyn HumaneInstruction>)
            .into_iter()
            .map(|i| {
                let segments =
                    parse_segments(i.segments()).expect("builtin instructions should be parseable");

                (segments, *i)
            }),
    )
}

/// Retrievers, used before a "should" clause
#[async_trait]
pub trait HumaneRetriever: Sync {
    fn segments(&self) -> &'static str;
    async fn run(
        &self,
        args: &SegmentArgs<'_>,
        civ: &mut Civilization,
    ) -> Result<serde_json::Value, HumaneStepError>;
}

inventory::collect!(&'static dyn HumaneRetriever);

pub fn register_retrievers() -> HashMap<HumaneSegments, &'static dyn HumaneRetriever> {
    HashMap::<_, _>::from_iter(
        (inventory::iter::<&dyn HumaneRetriever>)
            .into_iter()
            .map(|i| {
                let segments =
                    parse_segments(i.segments()).expect("builtin retrievers should be parseable");

                (segments, *i)
            }),
    )
}

/// Assertions, used after a "should" clause
#[async_trait]
pub trait HumaneAssertion: Sync {
    fn segments(&self) -> &'static str;
    async fn run(
        &self,
        base_value: serde_json::Value,
        args: &SegmentArgs<'_>,
        civ: &mut Civilization,
    ) -> Result<(), HumaneStepError>;
}

inventory::collect!(&'static dyn HumaneAssertion);

pub fn register_assertions() -> HashMap<HumaneSegments, &'static dyn HumaneAssertion> {
    HashMap::<_, _>::from_iter(
        (inventory::iter::<&dyn HumaneAssertion>)
            .into_iter()
            .map(|i| {
                let segments =
                    parse_segments(i.segments()).expect("builtin assertions should be parseable");

                (segments, *i)
            }),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_getting_an_instruction() {
        pub struct TestInstruction;

        inventory::submit! {
            &TestInstruction as &dyn HumaneInstruction
        }

        #[async_trait]
        impl HumaneInstruction for TestInstruction {
            fn segments(&self) -> &'static str {
                "I am an instruction asking for {argument}"
            }

            async fn run(
                &self,
                args: &SegmentArgs<'_>,
                civ: &mut Civilization,
            ) -> Result<(), HumaneStepError> {
                Ok(())
            }
        }

        let users_instruction = parse_segments("I am an instruction asking for \"this argument\"")
            .expect("Valid instruction");

        let all_instructions = register_instructions();
        let matching_instruction = all_instructions
            .get(&users_instruction)
            .expect("should be able to retrieve instruction");

        assert_eq!(
            matching_instruction.segments(),
            "I am an instruction asking for {argument}"
        );
    }

    #[test]
    fn test_getting_a_retriever() {
        pub struct TestRetriever;

        inventory::submit! {
            &TestRetriever as &dyn HumaneRetriever
        }

        #[async_trait]
        impl HumaneRetriever for TestRetriever {
            fn segments(&self) -> &'static str {
                "the file {filename}"
            }

            async fn run(
                &self,
                args: &SegmentArgs<'_>,
                civ: &mut Civilization,
            ) -> Result<serde_json::Value, HumaneStepError> {
                Ok(serde_json::Value::Null)
            }
        }

        let users_segments = parse_segments("the file \"index.html\"").expect("Valid instruction");

        let all_segments = register_retrievers();
        let matching_retriever = all_segments
            .get(&users_segments)
            .expect("should be able to retrieve segments");

        assert_eq!(matching_retriever.segments(), "the file {filename}");
    }

    #[test]
    fn test_getting_an_assertion() {
        pub struct TestAssertion;

        inventory::submit! {
            &TestAssertion as &dyn HumaneAssertion
        }

        #[async_trait]
        impl HumaneAssertion for TestAssertion {
            fn segments(&self) -> &'static str {
                "be exactly {value}"
            }

            async fn run(
                &self,
                base_value: serde_json::Value,
                args: &SegmentArgs<'_>,
                civ: &mut Civilization,
            ) -> Result<(), HumaneStepError> {
                Ok(())
            }
        }

        let users_segments = parse_segments("be exactly {my_json}").expect("Valid instruction");

        let all_segments = register_assertions();
        let matching_assertion = all_segments
            .get(&users_segments)
            .expect("should be able to retrieve segments");

        assert_eq!(matching_assertion.segments(), "be exactly {value}");
    }
}
