use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyPair {
    pub input_file: PathBuf,
    pub output_file: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct DependencyList {
    pub files: Vec<DependencyPair>,
}

impl DependencyList {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add(&mut self, input: &Path, output: &Path) {
        self.files.push(DependencyPair {
            input_file: input.to_path_buf(),
            output_file: output.to_path_buf(),
        });
    }

    pub fn remove_duplicates(&mut self) {
        self.files.sort();
        self.files.dedup();
    }
}
