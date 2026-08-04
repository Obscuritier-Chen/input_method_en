// crates/tsf-service/src/ipc_client.rs
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::sync::mpsc::{channel, Sender};
use std::thread;

use ime_protocol::{ClientRequest, ServerCommand};
use crate::window_bridge::WindowBridge;

const PIPE_NAME: &str = r"\\.\pipe\my_ime_named_pipe";

pub struct IpcClient {
    tx: Sender<ClientRequest>,
}

impl IpcClient {
    pub fn start(bridge: std::sync::Arc<WindowBridge>) -> Self {
        let (tx, rx) = channel::<ClientRequest>();

        thread::spawn(move || {
            // 打开管道文件（非阻塞尝试）
            let file = match OpenOptions::new()
                .read(true)
                .write(true)
                .open(PIPE_NAME)
            {
                Ok(f) => f,
                Err(_) => return, // 如果 Tauri 主进程未启动，静默退出，不卡死宿主应用
            };

            let mut reader = BufReader::new(file.try_clone().unwrap());
            let mut writer = file;

            // 1. 独立线程负责向 Tauri 发送消息
            thread::spawn(move || {
                while let Ok(req) = rx.recv() {
                    if let Ok(json) = serde_json::to_string(&req) {
                        let mut data = json.into_bytes();
                        data.push(b'\n');
                        if writer.write_all(&data).is_err() {
                            break;
                        }
                    }
                }
            });

            // 2. 当前线程循环读取 Tauri 下发的指令
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok() {
                if line.is_empty() {
                    break;
                }
                if let Ok(cmd) = serde_json::from_str::<ServerCommand>(&line) {
                    match cmd {
                        ServerCommand::CommitText { session_id: _, text } => {
                            // 收到选词，通过窗口桥接跨线程投递给主 STA 线程！
                            bridge.post_commit(text);
                        }
                        _ => {}
                    }
                }
                line.clear();
            }
        });

        Self { tx }
    }

    /// 发送输入状态更新请求给 Tauri
    pub fn send(&self, req: ClientRequest) {
        let _ = self.tx.send(req);
    }
}