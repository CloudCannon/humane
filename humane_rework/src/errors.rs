use std::path::PathBuf;

use pagebrowse_lib::PagebrowseError;
use thiserror::Error;

use crate::HumaneTestStep;

#[derive(Error, Debug)]
pub enum HumaneInputError {
    #[error("Argument {arg} was not supplied. (have [{has}])")]
    NonexistentArgument { arg: String, has: String },
    #[error("Argument {arg} expected to be a {expected}, but is a {was}")]
    IncorrectArgumentType {
        arg: String,
        was: String,
        expected: String,
    },
    #[error("Argument {arg} requires a value, cannot be empty")]
    ArgumentRequiresValue { arg: String },
    #[error("yaml failed to parse: {0}")]
    ParseError(#[from] serde_yaml::Error),
    #[error("unclosed argument, expected a {expected} character")]
    UnclosedValue { expected: char },
    #[error("invalid path: {input}")]
    InvalidPath { input: String },
    #[error("step does not exist")]
    NonexistentStep,
    #[error("step requirements were not met: {reason}")]
    StepRequirementsNotMet { reason: String },
}

#[derive(Error, Debug)]
pub enum HumaneInternalError {
    #[error("Test error: {msg}")]
    Custom { msg: String },
    #[error("{0}")]
    PagebrowseError(#[from] PagebrowseError),
}

#[derive(Error, Debug)]
pub enum HumaneTestFailure {
    #[error("Test failure: {msg}")]
    Custom { msg: String },
}

#[derive(Error, Debug)]
pub enum HumaneStepError {
    #[error("Parse error: {0}")]
    External(#[from] HumaneInputError),
    #[error("Step error: {0}")]
    Internal(#[from] HumaneInternalError),
    #[error("Failed assertion: {0}")]
    Assertion(#[from] HumaneTestFailure),
}

#[derive(Error, Debug)]
#[error("Error in step \"{step}\":\n{err}")]
pub struct HumaneTestError {
    pub err: HumaneStepError,
    pub step: HumaneTestStep,
}
