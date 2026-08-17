use tauri::webview::cookie::prefix;

use crate::AppState;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct CandidateDto {
    pub word: String,
    pub score: f32,
}

pub fn build_context(
    buffer: &str,
) -> Vec<String> {
    buffer
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

#[tauri::command]
pub fn generate_candidates(
    buffer: String,
    prefix: String,
    state: &AppState,
) -> Result<Vec<CandidateDto>, String> {
    //吗唔的gpt/gemini 和claude接口不一致
    let context= build_context(&prefix);
    let prefix = buffer;


    println!(
        "[GenerateCandidates] prefix='{}', context={:?}",
        prefix,
        context
    );

    let context_refs: Vec<&str> =
        context.iter().map(|s| s.as_str()).collect();

    let context_ids =
        state.vocab.encode(&context_refs);

    let candidate_ids = state
        .candidates
        .get_candidates(&prefix)
        .cloned()
        .unwrap_or_default();

    println!(
        "[GenerateCandidates] candidate count={}",
        candidate_ids.len()
    );

    if candidate_ids.is_empty() {
        println!(
            "[GenerateCandidates] no candidates for prefix '{}'",
            prefix
        );
        return Ok(vec![]);
    }

    let mut predictor = state
        .predictor
        .lock()
        .map_err(|e| e.to_string())?;

    let results = predictor
        .predict(
            &context_ids,
            &candidate_ids,
            &state.vocab,
            10,
        )
        .map_err(|e| e.to_string())?;

    for (rank, candidate) in results.iter().enumerate() {
        println!(
            "[Candidate #{:02}] word='{}', score={:.6}",
            rank + 1,
            candidate.word,
            candidate.score
        );
    }

    Ok(results
        .into_iter()
        .map(|c| CandidateDto {
            word: c.word,
            score: c.score,
        })
        .collect())
}