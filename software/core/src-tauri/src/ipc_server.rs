use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::{mpsc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

use ime_protocol::{ClientRequest, ServerCommand};
use crate::windows_utils::hide_candidate_window_native;
use crate::{AppState, generate_candidates};
use crate::sdll::SecurityAttributesGuard;
use crate::session::{SessionManager, SessionWriter};

pub const PIPE_NAME: &str = r"\\.\pipe\my_ime_named_pipe";


/// 启动 IPC Named Pipe 服务端后台任务
pub fn start_ipc_server(app_handle: AppHandle, state: SessionManager, app_state: Arc<AppState>) {
    println!("[IPC Server] 正在初始化管道服务, PipeName: {}", PIPE_NAME);
    tokio::spawn(async move {
        let mut first = true;
        loop {
            let server_res = {
                // SDDL 语法解释:
                // D: 显式 DACL
                // (A;;GA;;;WD) -> Allow (A) Generic All (GA) to World/Everyone (WD)
                // (A;;GA;;;AC) -> Allow (A) Generic All (GA) to All Application Packages / UWP (AC)
                let mut sa = match SecurityAttributesGuard::new_sddl("D:(A;;GA;;;WD)(A;;GA;;;AC)") {
                    Ok(sa) => Some(sa),
                    Err(e) => {
                        eprintln!("[IPC] 创建 SDDL 安全描述符失败: {:?}", e);
                        // 放弃这个结果，外层根据 None 处理 sleep
                        None
                    }
                };

                if let Some(ref mut sa) = sa {
                    // 在这个块内部完成 Windows API 的裸指针调用
                    unsafe {
                        ServerOptions::new()
                            .first_pipe_instance(first)
                            .create_with_security_attributes_raw(PIPE_NAME, sa.as_raw_ptr())
                            .ok() // 转换为 Option<NamedPipeServer>
                    }
                } else {
                    None
                }
            };
            let server = match server_res {
                Some(s) => s,
                None => {
                    eprintln!("[IPC] 管道创建失败，准备重试...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }
            };

            first = false;
            if server.connect().await.is_err() {
                continue;
            }

            let connection_id= state.next_connection_id();

            println!("[IPC Server] successfully connected to tsf, connection_id={}", connection_id);
            let app_handle_clone = app_handle.clone();
            let state_clone = state.clone();
            let app_state_clone = app_state.clone();

            tokio::spawn(async move {
                handle_client(server, connection_id, app_handle_clone, state_clone, app_state_clone).await;
            });
        }
    });
}

async fn handle_client(
    server: NamedPipeServer,
    connection_id: u64,
    app_handle: AppHandle,
    state: SessionManager,
    _app_state: Arc<AppState>,
) {
    // ============================================================
    // Duplex pipe
    // ============================================================

    let (read_half, mut write_half) = tokio::io::split(server);

    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    // ============================================================
    // Tauri -> TSF channel
    // ============================================================

    let (tx, mut rx) = mpsc::unbounded_channel::<ServerCommand>();

    let mut registered_session_id: Option<u32> = None;

    // ============================================================
    // Writer task
    // ============================================================

    let write_task = tokio::spawn(async move {
        println!("[IPC Writer] writer task started");

        while let Some(cmd) = rx.recv().await {
            println!(
                "[IPC Writer] received command from channel: {:?}",
                cmd
            );

            let json = match serde_json::to_string(&cmd) {
                Ok(json) => json,

                Err(e) => {
                    eprintln!(
                        "[IPC Writer] JSON serialization failed: {:?}",
                        e
                    );
                    continue;
                }
            };

            let mut data = json.into_bytes();
            data.push(b'\n');

            println!(
                "[IPC Writer] writing to TSF: {}",
                String::from_utf8_lossy(&data)
            );

            if let Err(e) = write_half.write_all(&data).await {
                eprintln!(
                    "[IPC Writer] pipe write failed: {:?}",
                    e
                );
                break;
            }

            println!(
                "[IPC Writer] write_all completed"
            );
        }

        println!(
            "[IPC Writer] writer task exited"
        );
    });

    // ============================================================
    // Reader loop: TSF -> Tauri
    // ============================================================

    loop {
        line.clear();

        println!(
            "[IPC Reader] waiting for data..."
        );

        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!(
                    "[IPC Reader] TSF disconnected"
                );
                break;
            }

            Ok(n) => {
                println!(
                    "[IPC Reader] received {} bytes: {:?}",
                    n,
                    line
                );

                let req =
                    match serde_json::from_str::<ClientRequest>(
                        line.trim_end()
                    ) {
                        Ok(req) => req,

                        Err(e) => {
                            eprintln!(
                                "[IPC Reader] JSON parse failed: {:?}",
                                e
                            );

                            continue;
                        }
                    };

                println!(
                    "[IPC Reader] parsed ClientRequest: {:?}",
                    req
                );

                match &req {
                    ClientRequest::UpdateContext {
                        session_id,
                        prefix,
                        buffer,
                        cursor_rect: _,
                    } => {
                        println!(
                            "[IPC] UpdateContext: session={}, prefix='{}', buffer='{}'",
                            session_id,
                            prefix,
                            buffer
                        );

                        // ---------------------------------------------
                        // 注册 session
                        // ---------------------------------------------

                        if registered_session_id != Some(*session_id) {
                            registered_session_id =
                                Some(*session_id);

                            let mut sessions =
                                state.sessions.lock().await;

                            sessions.insert(
                                *session_id,
                                SessionWriter {
                                    connection_id,
                                    tx: tx.clone(),
                                    candidates: Vec::new(),
                                },
                            );

                            println!(
                                "[IPC] Registered session {} -> connection {}",
                                session_id,
                                connection_id
                            );
                        }

                        // ---------------------------------------------
                        // Tauri -> frontend
                        // ---------------------------------------------

                        if let Err(e) = app_handle.emit(
                            "ime-update-context",
                            &req,
                        ) {
                            eprintln!(
                                "[IPC] emit failed: {:?}",
                                e
                            );
                        }
                    }

                    ClientRequest::CancelComposition {
                        session_id,
                    } => {
                        println!(
                            "[IPC] CancelComposition: session={}",
                            session_id
                        );

                        if let Err(e) = app_handle.emit(
                            "ime-cancel-composition",
                            &req,
                        ) {
                            eprintln!(
                                "[IPC] emit failed: {:?}",
                                e
                            );
                        }
                    }

                    ClientRequest::SelectCandidate { 
                        session_id, 
                        index 
                    } => {
                        println!("[IPC] SelectCandidate: session={}, index={}",session_id, index);

                        let word = match state
                            .get_candidate(
                                *session_id,
                                *index as usize,
                            )
                            .await{
                            Ok(word) => word,

                            Err(e) => {
                                eprintln!("[IPC] SelectCandidate failed: {}", e);
                                return;
                            }
                        };

                        println!("[IPC] SelectCandidate resolved: '{}'", word);

                        if let Some(window) = app_handle.get_webview_window("candidates")
                        {
                            if let Err(e) = hide_candidate_window_native(&window) {
                                eprintln!("[IPC] Failed to hide candidate window: {:?}", e);
                            }
                        }

                        let cmd =
                            ServerCommand::CommitText {
                                session_id: *session_id,
                                text: word,
                            };

                        if let Err(e) =
                            state
                                .send_to_session(
                                    *session_id,
                                    cmd,
                                )
                                .await{
                            eprintln!(
                                "[IPC] CommitText routing failed: {}", e);
                        }
                    }
                }
            }

            Err(e) => {
                eprintln!(
                    "[IPC Reader] pipe read failed: {:?}",
                    e
                );

                break;
            }
        }
    }

    // ============================================================
    // Cleanup
    // ============================================================

    if let Some(session_id) = registered_session_id {
        state
            .remove_session_if_owner(
                session_id,
                connection_id,
            )
            .await;
    }

    write_task.abort();

    println!(
        "[IPC] Connection {} handler exited",
        connection_id
    );
}