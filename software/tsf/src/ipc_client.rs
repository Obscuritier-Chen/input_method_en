use std::sync::Arc;
use std::thread;
use std::time::Duration;

use ime_protocol::{ClientRequest, ServerCommand};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::runtime::Builder;
use tokio::sync::mpsc;

use crate::window_bridge::WindowBridge;

use windows::core::PCWSTR;
use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;

const PIPE_NAME: &str = r"\\.\pipe\my_ime_named_pipe";

fn dbg(msg: &str) {
    let wide: Vec<u16> = msg
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        OutputDebugStringW(PCWSTR(wide.as_ptr()));
    }
}

pub struct IpcClient {
    tx: mpsc::UnboundedSender<ClientRequest>,
}

impl IpcClient {
    pub fn start(bridge: Arc<WindowBridge>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<ClientRequest>();

        thread::spawn(move || {
            dbg("[tsf] IPC Tokio runtime thread started");

            let runtime = Builder::new_current_thread()
                .enable_all()
                .build();

            match runtime {
                Ok(rt) => rt.block_on(run_ipc_client(bridge, rx)),
                Err(e) => dbg(&format!("[tsf] Failed to create Tokio runtime: {:?}", e)),
            }

            dbg("[tsf] IPC Tokio runtime thread exited");
        });

        Self { tx }
    }

    /// 发送输入状态更新请求给 Tauri（同步 API）
    pub fn send(&self, req: ClientRequest) {
        if let Err(e) = self.tx.send(req) {
            dbg(&format!("[tsf] ClientRequest send failed: {:?}", e));
        } else {
            dbg("[tsf] ClientRequest queued successfully");
        }
    }
}

async fn run_ipc_client(
    bridge: Arc<WindowBridge>,
    mut rx: mpsc::UnboundedReceiver<ClientRequest>,
) {
    loop {
        // 1. 尝试连接管道（卫语句循环，完全依赖类型推导）
        let client = loop {
            dbg("[tsf] Trying to connect to Tauri pipe...");
            match ClientOptions::new().open(PIPE_NAME) {
                Ok(c) => {
                    dbg("[tsf] Successfully connected to Tauri server pipe!");
                    break c;
                }
                Err(err) => {
                    dbg(&format!("[tsf] Failed to connect to Tauri pipe: {:?}, retrying in 1s...", err));
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        };

        dbg("[tsf] NamedPipeClient established");

        // 2. 拆分读写半区
        let (read_half, mut write_half) = tokio::io::split(client);
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();

        dbg("[tsf] IPC duplex read/write started");

        // 3. 事件双工处理主循环
        loop {
            line.clear();

            tokio::select! {
                // A. 处理来自 Tauri 的消息
                read_res = reader.read_line(&mut line) => {
                    let bytes_read = match read_res {
                        Ok(0) => {
                            dbg("[tsf] Tauri closed pipe connection");
                            break;
                        }
                        Ok(n) => n,
                        Err(e) => {
                            dbg(&format!("[tsf] Pipe read failed: {:?}", e));
                            break;
                        }
                    };

                    let raw_line = line.trim_end();
                    dbg(&format!("[tsf] Received {} bytes from Tauri: {}", bytes_read, raw_line));

                    if let Ok(cmd) = serde_json::from_str::<ServerCommand>(raw_line) {
                        match cmd {
                            ServerCommand::CommitText { session_id: _, text } => {
                                dbg(&format!("[tsf] Triggering CommitText: {}", text));
                                bridge.post_commit(text);
                            }
                            _ => dbg("[tsf] Received unsupported ServerCommand"),
                        }
                    } else {
                        dbg("[tsf] Failed to parse ServerCommand");
                    }
                }

                // B. 处理发往 Tauri 的消息
                req = rx.recv() => {
                    let req = match req {
                        Some(r) => r,
                        None => {
                            dbg("[tsf] ClientRequest channel closed");
                            return; // MPSC 关闭，彻底退出线程
                        }
                    };

                    let json = match serde_json::to_string(&req) {
                        Ok(j) => j,
                        Err(e) => {
                            dbg(&format!("[tsf] serde error: {:?}", e));
                            continue;
                        }
                    };

                    dbg(&format!("[tsf] sending JSON to pipe: {}", json));

                    let mut data = json.into_bytes();
                    data.push(b'\n');

                    if let Err(e) = write_half.write_all(&data).await {
                        dbg(&format!("[tsf] Pipe write failed: {:?}", e));
                    } else {
                        dbg("[tsf] async write_all completed");
                    }
                }
            }
        }

        dbg("[tsf] IPC connection lost, reconnecting...");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}