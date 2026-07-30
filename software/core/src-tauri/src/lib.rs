mod vocabulary;
mod candidate_index;
mod inference;

use std::path::Path;
use std::sync::Mutex;

pub use vocabulary::WordVocabulary;
pub use candidate_index::CandidateIndex;
pub use inference::Predictor;

struct AppState {
    vocab: WordVocabulary,
    candidates: CandidateIndex,
    predictor: Mutex<Predictor>,
}

#[derive(serde::Serialize)]
struct CandidateDto {
    word: String,
    score: f32,
}

#[tauri::command]
fn get_candidates(
    context: Vec<String>,
    prefix: String,
    state: tauri::State<AppState>,
) -> Result<Vec<CandidateDto>, String> {

    let context_refs: Vec<&str> = context.iter().map(|s| s.as_str()).collect();
    let context_ids = state.vocab.encode(&context_refs);

    let candidate_ids = state
        .candidates
        .get_candidates(&prefix)
        .cloned()
        .unwrap_or_default();

    if candidate_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut predictor = state
        .predictor
        .lock()
        .map_err(|e| e.to_string())?;

    let results = predictor
        .predict(&context_ids, &candidate_ids, &state.vocab, 10)
        .map_err(|e| e.to_string())?;

    Ok(results
        .into_iter()
        .map(|c| CandidateDto { word: c.word, score: c.score })
        .collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {

    // 用 CARGO_MANIFEST_DIR 拼绝对路径，避免 `tauri dev` 运行时工作目录
    // 和你预期不一致导致相对路径找不到文件
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let vocab_path = Path::new(manifest_dir)
        .join("../../../model/datasets/vocabulary/word2id.json");

    let candidate_path = Path::new(manifest_dir)
        .join("../../../model/datasets/candidate/prefix_candidates.bin");

    let onnx_path = Path::new(manifest_dir)
        .join("../../../model_export/output/completion_model_v3.onnx");

    let vocab = WordVocabulary::load(&vocab_path)
        .expect("加载 word2id.json 失败");

    let candidates = CandidateIndex::load_binary(&candidate_path)
        .expect("加载 prefix_candidates.bin 失败");

    let predictor = Predictor::load(&onnx_path)
        .expect("加载 ONNX 模型失败");

    tauri::Builder::default()
        .manage(AppState {
            vocab,
            candidates,
            predictor: Mutex::new(predictor),
        })
        .invoke_handler(tauri::generate_handler![get_candidates])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}