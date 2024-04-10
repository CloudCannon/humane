use pagebrowse_lib::Pagebrowser;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Arc,
};

use crate::{
    definitions::{HumaneAssertion, HumaneInstruction, HumaneRetriever},
    options::HumaneContext,
    segments::HumaneSegments,
    HumaneTestFile,
};

pub struct Universe<'u> {
    pub pagebrowser: Arc<Pagebrowser>,
    pub tests: BTreeMap<PathBuf, HumaneTestFile>,
    pub instructions: HashMap<HumaneSegments, &'u dyn HumaneInstruction>,
    pub instruction_comparisons: Vec<String>,
    pub retrievers: HashMap<HumaneSegments, &'u dyn HumaneRetriever>,
    pub retriever_comparisons: Vec<String>,
    pub assertions: HashMap<HumaneSegments, &'u dyn HumaneAssertion>,
    pub assertion_comparisons: Vec<String>,
    pub ctx: HumaneContext,
}
