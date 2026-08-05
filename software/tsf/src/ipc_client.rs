// crates/tsf-service/src/ipc_client.rs
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;

use ime_protocol::{ClientRequest, ServerCommand};
use crate::window_bridge::WindowBridge;

const PIPE_NAME: &str = r"\\.\pipe\my_ime_named_pipe";

use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
use windows::core::PCWSTR;

fn dbg(msg: &str) {
    let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { OutputDebugStringW(PCWSTR(wide.as_ptr())) };
}

pub struct IpcClient {
    tx: Sender<ClientRequest>,
}

impl IpcClient {
    pub fn start(bridge: std::sync::Arc<WindowBridge>) -> Self {
        let (tx, rx) = channel::<ClientRequest>();

        thread::spawn(move || {
            dbg("[tsf] IPC background listener thread started");

            // 建立连接重试循环 (Connect Retry Loop)
            let file = loop {
                match OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(PIPE_NAME)
                {
                    Ok(f) => {
                        dbg("[tsf] Successfully connected to Tauri server pipe!");
                        break f;
                    }
                    Err(err) => {
                        // 失败时记录日志并等待，防止卡死 UI
                        dbg(&format!(
                            "[tsf-ipc] Failed to connect to Tauri pipe: {:?} retrying in 1s...",
                            err
                        ));
                        thread::sleep(Duration::from_secs(10));
                    }
                }
            };

            // 2. 管道连通后，克隆句柄分别用于读写
            let mut reader = BufReader::new(match file.try_clone() {
                Ok(f) => f,
                Err(e) => {
                    dbg(&format!("[tsf-ipc] Failed to clone pipe handle: {:?}", e));
                    return;
                }
            });
            let mut writer = file;

            // 3. 独立线程负责向 Tauri 发送消息
            thread::spawn(move || {
                dbg("[tsf] Writer thread started");
                while let Ok(req) = rx.recv() {
                    if let Ok(json) = serde_json::to_string(&req) {
                        let mut data = json.into_bytes();
                        data.push(b'\n');
                        if let Err(e) = writer.write_all(&data) {
                            dbg(&format!("[tsf] Failed to send message to Tauri: {:?}", e));
                            break;
                        }
                    }
                }
                dbg("[tsf] Writer thread exited");
            });

            // 4. 当前线程循环读取 Tauri 下发的指令
            dbg("[tsf] Listening for incoming commands from Tauri...");
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok() {
                if line.is_empty() {
                    dbg("[tsf] Received EOF, Tauri disconnected the pipe");
                    break;
                }
                
                dbg(&format!("[tsf] received server command: {}", line.trim()));

                if let Ok(cmd) = serde_json::from_str::<ServerCommand>(&line) {
                    match cmd {
                        ServerCommand::CommitText { session_id: _, text } => {
                            dbg(&format!("[tsf] Triggering CommitText: {}", text));
                            bridge.post_commit(text);
                        }
                        _ => {}
                    }
                }
                line.clear();
            }
            dbg("[tsf] Reader thread exited");
        });

        Self { tx }
    }

    /// 发送输入状态更新请求给 Tauri
    pub fn send(&self, req: ClientRequest) {
        let _ = self.tx.send(req);
    }
}