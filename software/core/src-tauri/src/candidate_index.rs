use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Result;

pub struct CandidateIndex {
    table: HashMap<String, Vec<i64>>,
}

impl CandidateIndex {

    pub fn load_binary(path: impl AsRef<Path>) -> Result<Self> {

        let bytes = fs::read(path)?;

        let table: HashMap<String, Vec<i64>> = bincode::deserialize(&bytes)?;

        Ok(Self { table })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {

        let text = fs::read_to_string(path)?;

        let table: HashMap<String, Vec<i64>> = serde_json::from_str(&text)?;

        Ok(Self { table })
    }

    pub fn get_candidates(&self, prefix: &str) -> Option<&Vec<i64>> {

        self.table.get(prefix)
    }

    pub fn len(&self) -> usize {

        self.table.len()
    }
}