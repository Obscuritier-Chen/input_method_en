use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use tokio::sync::{
    mpsc,
    Mutex,
};

use ime_protocol::ServerCommand;

use crate::CandidateDto;


#[derive(Clone)]
pub struct SessionWriter {
    pub connection_id: u64,
    pub tx: mpsc::UnboundedSender<ServerCommand>,
    pub candidates: Vec<String>,
}
/// 管理所有活动 TSF 客户端连接的 Session Map
#[derive(Clone)]
pub struct SessionManager {
    pub sessions: Arc<Mutex<HashMap<u32, SessionWriter>>>,
    next_connection_id: Arc<AtomicU64>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            next_connection_id: Arc::new(AtomicU64::new(1)),
        }
    }
}

impl SessionManager {
    pub fn next_connection_id(&self) -> u64 {
        self.next_connection_id
            .fetch_add(1, Ordering::Relaxed)
    }

    /// 向指定 session 的当前 writer 发送 ServerCommand。
    pub async fn send_to_session(
        &self,
        session_id: u32,
        cmd: ServerCommand,
    ) -> Result<(), String> {
        let tx = {
            let sessions = self.sessions.lock().await;

            sessions
                .get(&session_id)
                .map(|writer| writer.tx.clone())
        };

        match tx {
            Some(tx) => {
                tx.send(cmd)
                    .map_err(|_| {
                        format!(
                            "writer channel for session {} is closed",
                            session_id
                        )
                    })
            }

            None => Err(format!(
                "session {} is not registered",
                session_id
            )),
        }
    }

    /// 只有当指定 connection 仍然拥有该 session 时，
    /// 才允许删除映射。
    pub async fn remove_session_if_owner(
        &self,
        session_id: u32,
        connection_id: u64,
    ) {
        let mut sessions = self.sessions.lock().await;

        let should_remove = sessions
            .get(&session_id)
            .map(|writer| writer.connection_id == connection_id)
            .unwrap_or(false);

        if should_remove {
            sessions.remove(&session_id);

            println!(
                "[IPC] Removed session {} from connection {}",
                session_id,
                connection_id
            );
        } else {
            println!(
                "[IPC] Skip stale cleanup: session {} is no longer owned by connection {}",
                session_id,
                connection_id
            );
        }
    }

    pub async fn set_candidates(
        &self,
        session_id: u32,
        candidates: Vec<String>,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;

        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(|| {
                format!(
                    "session {} not found",
                    session_id
                )
            })?;

        session.candidates =
            candidates.into_iter().take(5).collect();

        Ok(())
    }

    pub async fn get_candidate(
        &self,
        session_id: u32,
        index: usize,
    ) -> Result<String, String> {
        let sessions =
            self.sessions.lock().await;

        let session = sessions
            .get(&session_id)
            .ok_or_else(|| {
                format!(
                    "session {} not found",
                    session_id
                )
            })?;

        let candidate = session
            .candidates
            .get(index)
            .ok_or_else(|| {
                format!(
                    "candidate index {} out of range for session {}",
                    index,
                    session_id
                )
            })?;

        Ok(candidate.clone())
    }
}