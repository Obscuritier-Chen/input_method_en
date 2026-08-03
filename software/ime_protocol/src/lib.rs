// crates/ime-protocol/src/lib.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// TSF DLL -> Tauri 主进程 发送的消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientRequest {
    /// 输入状态更新（携带会话 ID、前文、当前 typed buffer、光标坐标）
    UpdateContext {
        session_id: u32,
        prefix: String,
        buffer: String,
        cursor_rect: Option<CursorRect>,
    },
    /// 取消输入 / 组合态被清空
    CancelComposition { session_id: u32 },
}

/// Tauri 主进程 -> TSF DLL 反向下发的指令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerCommand {
    /// 要求 TSF 提交（Commit）指定的候选词到目标应用光标处
    CommitText {
        session_id: u32,
        text: String,
    },
}