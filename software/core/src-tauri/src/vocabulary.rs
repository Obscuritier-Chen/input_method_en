use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Result;

pub struct WordVocabulary {
    word_to_id: HashMap<String, i64>,
    id_to_word: Vec<String>,
    unk_id: i64,
}

impl WordVocabulary {

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {

        let text = fs::read_to_string(path)?;

        let word_to_id: HashMap<String, i64> = serde_json::from_str(&text)?;

        let vocab_size = word_to_id
            .values()
            .max()
            .copied()
            .unwrap_or(0) as usize + 1;

        let mut id_to_word = vec![String::new(); vocab_size];

        for (word, &id) in &word_to_id {
            if let Some(slot) = id_to_word.get_mut(id as usize) {
                *slot = word.clone();
            }
        }

        // 假设词表里有 <unk> 这个特殊 token，如果实际训练时用的是别的写法
        // （比如 [UNK] 或 unk），这里需要相应调整
        let unk_id = *word_to_id.get("<unk>").unwrap_or(&0);

        Ok(Self {
            word_to_id,
            id_to_word,
            unk_id,
        })
    }

    pub fn encode(&self, words: &[&str]) -> Vec<i64> {

        words
            .iter()
            .map(|w| self.word_to_id(w))
            .collect()
    }

    pub fn word_to_id(&self, word: &str) -> i64 {

        *self.word_to_id.get(word).unwrap_or(&self.unk_id)
    }

    pub fn id_to_word(&self, id: i64) -> &str {

        self.id_to_word
            .get(id as usize)
            .map(|s| s.as_str())
            .unwrap_or("<unk>")
    }

    pub fn len(&self) -> usize {

        self.id_to_word.len()
    }
}