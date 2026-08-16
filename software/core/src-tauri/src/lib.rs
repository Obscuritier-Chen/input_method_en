mod vocabulary;
mod candidate_index;
mod inference;
mod ipc_server;
mod sdll;
mod session;

pub use vocabulary::WordVocabulary;
pub use candidate_index::CandidateIndex;
pub use inference::{Predictor, Candidate};
pub use session::SessionManager;

use std::path::Path;
use std::sync::{Mutex, Arc};

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, Emitter, State};

use ime_protocol::ServerCommand;

use crate::ipc_server::start_ipc_server;

struct AppState {
    vocab: WordVocabulary,
    candidates: CandidateIndex,
    predictor: Mutex<Predictor>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct CandidateDto {
    word: String,
    score: f32,
}

#[tauri::command]
async fn get_candidates(
    context: Vec<String>,
    prefix: String,
    state: tauri::State<'_ , AppState>,
    session_id: u32,
    pipe_state: State<'_, SessionManager>, 
    app_handle: tauri::AppHandle,
) -> Result<Vec<CandidateDto>, String> {

    println!("[Debug get_candidates] Received request -> prefix: '{prefix}', context: {context:?}");

    // 2. 跳过选词过程：默认直接选择 "test" 并自动提交上屏
    println!("[Debug Auto-Select] Skipping candidate selection, auto-committing 'test'...");
    
    // 自动调用已有的选词提交逻辑
    let _ = on_candidate_selected(session_id, "test".to_string(), pipe_state, app_handle).await;

    // 3. 返回空列表（或模拟候选词），此时 UI 窗口已经被自动隐藏
    Ok(vec![])

    /*let context_refs: Vec<&str> = context.iter().map(|s| s.as_str()).collect();
    let context_ids = state.vocab.encode(&context_refs);

    let candidate_ids = state
        .candidates
        .get_candidates(&prefix)
        .cloned()
        .unwrap_or_default();

    if candidate_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut predictor = state.predictor.lock().map_err(|e| e.to_string())?;

    let results = predictor
        .predict(&context_ids, &candidate_ids, &state.vocab, 10)
        .map_err(|e| e.to_string())?;

    Ok(results
        .into_iter()
        .map(|c| CandidateDto { word: c.word, score: c.score })
        .collect())*/
}

#[tauri::command]
fn show_candidates_window(
    app: tauri::AppHandle,
    x: f64,
    y: f64,
    candidates: Vec<CandidateDto>,
) -> Result<(), String> {

    let window = app
        .get_webview_window("candidates")
        .ok_or("candidate window not found")?;

    window
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;

    window
        .emit("candidates-updated", &candidates)
        .map_err(|e| e.to_string())?;

    window.show().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn hide_candidates_window(app: tauri::AppHandle) -> Result<(), String> {

    if let Some(window) = app.get_webview_window("candidates") {
        window.hide().map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn on_candidate_selected(
    session_id: u32,
    word: String,
    state: State<'_, SessionManager>,
    app_handle: tauri::AppHandle
) -> Result<(), String> {
    let cmd = ServerCommand::CommitText {
        session_id,
        text: word,
    };

    // 通过 session_id 路由到对应 TSF connection 的 writer
    state
        .send_to_session(session_id, cmd)
        .await?;

    // 选完词后自动隐藏候选窗口
    if let Some(window) = app_handle.get_webview_window("candidates") {
        let _ = window.hide();
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {

    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let vocab_path = Path::new(manifest_dir).join("../../../model/datasets/vocabulary/word2id.json");
    let candidate_path = Path::new(manifest_dir).join("../../../model/datasets/candidate/prefix_candidates.bin");
    let onnx_path = Path::new(manifest_dir).join("../../../model_export/output/completion_model_v3.onnx");

    let vocab = WordVocabulary::load(&vocab_path).expect("加载 word2id.json 失败");
    let candidates = CandidateIndex::load_binary(&candidate_path).expect("加载 prefix_candidates.bin 失败");
    let predictor = Predictor::load(&onnx_path).expect("加载 ONNX 模型失败");

    let app_state = Arc::new(AppState {
        vocab,
        candidates,
        predictor: Mutex::new(predictor),
    });
    // 1. 初始化 IPC 管道状态
    let pipe_state = SessionManager::default();

    /*tauri::Builder::default()
        .manage(AppState {
            vocab,
            candidates,
            predictor: Mutex::new(predictor),
        })
        .manage(pipe_state.clone())
        .setup(move |app| {
            start_ipc_server(app.handle().clone(), pipe_state);

            WebviewWindowBuilder::new(app, "candidates", WebviewUrl::App("candidate.html".into()))
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .visible(false)
                .inner_size(200.0, 40.0)
                .build()?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_candidates,
            show_candidates_window,
            hide_candidates_window,
            on_candidate_selected
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");*/
    tauri::Builder::default()
        .manage(app_state.clone())
        .manage(pipe_state.clone())
        .setup(move |app| {
            // 2. 将 app_state 传入管道后台任务，完全脱离前端独立运行
            start_ipc_server(app.handle().clone(), pipe_state, app_state);

            WebviewWindowBuilder::new(app, "candidates", WebviewUrl::App("candidate.html".into()))
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .visible(false)
                .inner_size(200.0, 40.0)
                .build()?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_candidates,
            show_candidates_window,
            hide_candidates_window,
            on_candidate_selected
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}