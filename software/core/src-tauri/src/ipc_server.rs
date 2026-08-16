// src-tauri/src/ipc_server.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::{mpsc, Mutex};
use tauri::{AppHandle, Emitter};

use ime_protocol::{ClientRequest, ServerCommand};
use crate::AppState;
use crate::sdll::SecurityAttributesGuard;

pub const PIPE_NAME: &str = r"\\.\pipe\my_ime_named_pipe";

/// 管理所有活动 TSF 客户端连接的 Session Map
#[derive(Default, Clone)]
pub struct PipeServerState {
    pub sessions: Arc<Mutex<HashMap<u32, mpsc::UnboundedSender<ServerCommand>>>>,
}

/// 启动 IPC Named Pipe 服务端后台任务
pub fn start_ipc_server(app_handle: AppHandle, state: PipeServerState, app_state: Arc<AppState>) {
    println!("🟢 [IPC Server] 正在初始化管道服务, PipeName: {}", PIPE_NAME);
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

            println!("🤝 [IPC Server] 成功监听到来自 TSF DLL 的管道连接！");
            let app_handle_clone = app_handle.clone();
            let state_clone = state.clone();
            let app_state_clone = app_state.clone();

            tokio::spawn(async move {
                handle_client(server, app_handle_clone, state_clone, app_state_clone).await;
            });
        }
    });
}

/// 单个 TSF 客户端连接的处理逻辑
/*async fn handle_client(
    server: NamedPipeServer,
    app_handle: AppHandle,
    state: PipeServerState,
    app_state: Arc<AppState>
) {
    /*let (read_half, mut write_half) = tokio::io::split(server);
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    // 创建反向向客户端发送指令的 Channel
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerCommand>();
    let mut registered_session_id: Option<u32> = None;

    // 启动反向写入 Task (向 TSF 下发指令)
    let write_task = tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&cmd) {
                let mut data = json.into_bytes();
                data.push(b'\n'); // 增加换行符作为帧分割标志
                if write_half.write_all(&data).await.is_err() {
                    break;
                }
            }
        }
    });

    // 循环读取 TSF 客户端上报的消息
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF, 客户端断开连接
            Ok(_) => {
                if let Ok(req) = serde_json::from_str::<ClientRequest>(&line) {
                    match &req {
                        ClientRequest::UpdateContext { session_id, prefix: _, buffer: _, cursor_rect: _ } => {
                            // 动态注册 Session ID 与 Channel 映射
                            if registered_session_id != Some(*session_id) {
                                registered_session_id = Some(*session_id);
                                state.sessions.lock().await.insert(*session_id, tx.clone());
                            }

                            // 1. 发送给 Tauri 前端 WebView (用于渲染候选框 & 移动光标位置)
                            let _ = app_handle.emit("ime-update-context", &req);

                            // TODO: 在这里可以触发你的 Rust 算法/模型推理逻辑代码
                        }
                        ClientRequest::CancelComposition { session_id: _ } => {
                            // 通知前端隐藏候选窗口
                            let _ = app_handle.emit("ime-cancel-composition", &req);
                        }
                    }
                }
            }
            Err(_) => break, // 读取出错（如管道破裂）
        }
    }

    // 客户端断开连接，清理 Session 映射
    if let Some(session_id) = registered_session_id {
        state.sessions.lock().await.remove(&session_id);
    }
    write_task.abort();*/

    //temp below
    let (read_half, mut write_half) = tokio::io::split(server);
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    let (tx, mut rx) = mpsc::unbounded_channel::<ServerCommand>();
    let mut registered_session_id: Option<u32> = None;

    // 后台写入线程
    let write_task = tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&cmd) {
                let mut data = json.into_bytes();
                data.push(b'\n');
                if write_half.write_all(&data).await.is_err() {
                    break;
                }
            }
        }
    });

    // 循环接收 TSF 上报请求
    loop {
        line.clear();
        println!("[IPC] waiting for data");

        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!("[IPC] cliend disconnected");
                break;
            }, // 客户端断开
            Ok(n) => {
                println!("[IPC Received {} bytes]: {}", n, line.trim());
                if let Ok(req) = serde_json::from_str::<ClientRequest>(&line) {
                    match req {
                        ClientRequest::UpdateContext { session_id, prefix, buffer, cursor_rect: _ } => {
                            println!("\n🔥 [Backend Direct] 收到 TSF 请求 -> session_id: {}, prefix: '{}', buffer: '{}'", session_id, prefix, buffer);

                            // 1. 注册 Session
                            if registered_session_id != Some(session_id) {
                                registered_session_id = Some(session_id);
                                state.sessions.lock().await.insert(session_id, tx.clone());
                            }

                            // 2. 【脱离前端测试 A】：直接自动回传硬编码字符串 "test"
                            println!("🚀 [Backend Direct] 绕过前端，直接向 TSF 管道发送 CommitText ('test')...");
                            let cmd = ServerCommand::CommitText {
                                session_id,
                                text: "test".to_string(),
                            };
                            let _ = tx.send(cmd);

                            // 3. 【脱离前端测试 B】：直接在此进行 Rust 原生 ONNX 模型推理 (解除下方注释即可测试模型)
                            /*
                            let context_refs = vec![prefix.as_str()];
                            let context_ids = app_state.vocab.encode(&context_refs);
                            if let Some(candidate_ids) = app_state.candidates.get_candidates(&buffer) {
                                if let Ok(mut predictor) = app_state.predictor.lock() {
                                    if let Ok(results) = predictor.predict(&context_ids, candidate_ids, &app_state.vocab, 1) {
                                        if let Some(top1) = results.first() {
                                            println!("🤖 [Model Inference] 预测 Top1 词汇: {}", top1.word);
                                            let cmd = ServerCommand::CommitText {
                                                session_id,
                                                text: top1.word.clone(),
                                            };
                                            let _ = tx.send(cmd);
                                        }
                                    }
                                }
                            }
                            */
                        }
                        ClientRequest::CancelComposition { session_id } => {
                            println!("[Backend Direct] 收到 CancelComposition -> session_id: {}", session_id);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[IPC] read failed {:?}", e);
                break;
            },
        }
    }

    if let Some(session_id) = registered_session_id {
        state.sessions.lock().await.remove(&session_id);
    }
    write_task.abort();
}*/
async fn handle_client(
    server: NamedPipeServer,
    app_handle: AppHandle,
    state: PipeServerState,
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
                            registered_session_id = Some(*session_id);

                            state
                                .sessions
                                .lock()
                                .await
                                .insert(*session_id, tx.clone());

                            println!(
                                "[IPC] session {} registered",
                                session_id
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

                        // ---------------------------------------------
                        // TEST:
                        //
                        // Tauri -> TSF
                        // ---------------------------------------------

                        println!(
                            "[IPC TEST] queue CommitText('test')"
                        );

                        let cmd = ServerCommand::CommitText {
                            session_id: *session_id,
                            text: "test".to_string(),
                        };

                        match tx.send(cmd) {
                            Ok(()) => {
                                println!(
                                    "[IPC TEST] CommitText queued"
                                );
                            }

                            Err(e) => {
                                eprintln!(
                                    "[IPC TEST] CommitText queue failed: {:?}",
                                    e
                                );
                            }
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
            .sessions
            .lock()
            .await
            .remove(&session_id);

        println!(
            "[IPC] Removed session {}",
            session_id
        );
    }

    write_task.abort();

    println!(
        "[IPC] handle_client exited"
    );
}