mod vocabulary;
mod candidate_index;
mod inference;
mod ipc_server;
mod sdll;
mod session;
mod completion_service;

pub use vocabulary::WordVocabulary;
pub use candidate_index::CandidateIndex;
pub use inference::{Predictor, Candidate};
pub use session::SessionManager;
pub use completion_service::{
    build_context,
    generate_candidates,
    CandidateDto,
};

use std::path::Path;
use std::sync::{Mutex, Arc};

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, Emitter, State};

use ime_protocol::ServerCommand;

use crate::ipc_server::start_ipc_server;

pub struct AppState {
    vocab: WordVocabulary,
    candidates: CandidateIndex,
    predictor: Mutex<Predictor>,
}

#[tauri::command]
async fn get_candidates(
    buffer: String,
    prefix: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<CandidateDto>, String> {

    generate_candidates(
        buffer,
        prefix,
        state.as_ref(),
    )
}

#[tauri::command]
fn show_candidates_window(
    app: tauri::AppHandle,
    x: f64,
    y: f64,
) -> Result<(), String> {

    let window = app
        .get_webview_window("candidates")
        .ok_or("candidate window not found")?;

    window
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;

    window.show().map_err(|e| e.to_string())?;

    window.set_focus().map_err(|e| e.to_string())?;

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

    tauri::Builder::default()
        .manage(app_state.clone())
        .manage(pipe_state.clone())
        .setup(move |app| {
            let window= WebviewWindowBuilder::new(
                app,
                "candidates",
                WebviewUrl::App("candidate.html".into()),
            )
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .focusable(true)
            .visible(false)
            .inner_size(240.0, 200.0)
            .build()?;

            start_ipc_server(
                app.handle().clone(),
                pipe_state,
                app_state,
            );

            #[cfg(debug_assertions)]
            window.open_devtools();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_candidates,
            show_candidates_window,
            hide_candidates_window,
            on_candidate_selected,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}