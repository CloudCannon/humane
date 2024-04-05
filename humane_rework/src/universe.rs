use pagebrowse_lib::Pagebrowser;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{
    instructions::{HumaneInstruction, HumaneSegments},
    HumaneTestFile,
};

pub struct Universe<'u> {
    pub pagebrowser: Arc<Pagebrowser>,
    pub tests: HashMap<PathBuf, HumaneTestFile>,
    pub instructions: HashMap<HumaneSegments, &'u dyn HumaneInstruction>,
    pub instruction_comparisons: Vec<String>,
}
