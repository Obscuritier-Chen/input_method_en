// src-tauri/src/ipc_server.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::{mpsc, Mutex};
use tauri::{AppHandle, Emitter};

use ime_protocol::{ClientRequest, ServerCommand};
use crate::sdll::SecurityAttributesGuard;

pub const PIPE_NAME: &str = r"\\.\pipe\my_ime_named_pipe";

/// 管理所有活动 TSF 客户端连接的 Session Map
#[derive(Default, Clone)]
pub struct PipeServerState {
    pub sessions: Arc<Mutex<HashMap<u32, mpsc::UnboundedSender<ServerCommand>>>>,
}

/// 启动 IPC Named Pipe 服务端后台任务
pub fn start_ipc_server(app_handle: AppHandle, state: PipeServerState) {
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

            let app_handle_clone = app_handle.clone();
            let state_clone = state.clone();

            tokio::spawn(async move {
                handle_client(server, app_handle_clone, state_clone).await;
            });
        }
    });
}

/// 单个 TSF 客户端连接的处理逻辑
async fn handle_client(
    server: NamedPipeServer,
    app_handle: AppHandle,
    state: PipeServerState,
) {
    let (read_half, mut write_half) = tokio::io::split(server);
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
    write_task.abort();
}