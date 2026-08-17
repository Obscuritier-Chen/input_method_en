use std::path::Path;

use ort::session::Session;
use ort::value::Tensor;

use anyhow::Result;

use crate::vocabulary::WordVocabulary;

pub struct Predictor {
    session: Session,
}

pub struct Candidate {
    pub word: String,
    pub score: f32,
}

impl Predictor {

    pub fn load(onnx_path: impl AsRef<Path>) -> Result<Self> {

        let session = Session::builder()?
            .commit_from_file(onnx_path)?;

        Ok(Self { session })
    }

    pub fn predict(

        &mut self,

        context_ids: &[i64],

        candidate_ids: &[i64],

        vocab: &WordVocabulary,

        top_k: usize,

    ) -> Result<Vec<Candidate>> {

        let context_ids = if context_ids.is_empty() {//应对context empty 的临时不严谨方案
            vec![0i64]
        } else {
            context_ids.to_vec()
        };

        let context_len = context_ids.len();
        let candidate_count = candidate_ids.len();

        let context_tensor = Tensor::from_array((
            [1usize, context_len],
            context_ids.to_vec(),
        ))?;

        let candidate_tensor = Tensor::from_array((
            [1usize, candidate_count],
            candidate_ids.to_vec(),
        ))?;

        let mask: Vec<bool> = vec![true; candidate_count];

        let mask_tensor = Tensor::from_array((
            [1usize, candidate_count],
            mask,
        ))?;

        let outputs = self.session.run(ort::inputs![
            "context_ids" => context_tensor,
            "candidate_ids" => candidate_tensor,
            "candidate_mask" => mask_tensor,
        ])?;

        let logits = outputs["logits"].try_extract_tensor::<f32>()?;

        let scores = logits.1;

        let mut ranked: Vec<(usize, f32)> = scores
            .iter()
            .copied()
            .enumerate()
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let result = ranked
            .into_iter()
            .take(top_k)
            .map(|(idx, score)| Candidate {
                word: vocab.id_to_word(candidate_ids[idx]).to_string(),
                score,
            })
            .collect();

        Ok(result)
    }
}