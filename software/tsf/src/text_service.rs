use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::fmt;

use windows::core::{implement, IUnknown, Result, Interface};
use windows::Win32::UI::TextServices::{
    ITfTextInputProcessor, ITfTextInputProcessor_Impl,
    ITfThreadMgr, ITfKeyEventSink, ITfKeyEventSink_Impl,
    ITfKeystrokeMgr,ITfComposition, ITfContext,
    ITfEditSession, ITfEditSession_Impl,
};
use windows::Win32::UI::TextServices::{
    TF_ES_SYNC,
    TF_ES_READWRITE,
};
use windows::Win32::Foundation::{BOOL, WPARAM, LPARAM, TRUE, FALSE};

use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
use windows::core::PCWSTR;

fn dbg(msg: &str) {
    let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { OutputDebugStringW(PCWSTR(wide.as_ptr())) };
}

use crate::edit_session::{KeyEditSession, KeyAction};
use crate::window_bridge::WindowBridge;
use crate::ipc_client::IpcClient;

#[derive(Debug, Clone)]

#[implement(ITfTextInputProcessor, ITfKeyEventSink)]
pub struct TextService {
    thread_mgr: RefCell<Option<ITfThreadMgr>>,
    client_id: std::cell::Cell<u32>,
    state: Rc<SharedState>,
}

pub struct SharedState {
    pub client_id: std::cell::Cell<u32>,
    pub composition: RefCell<Option<ITfComposition>>,
    pub buffer: RefCell<String>,
    pub ipc_client: RefCell<Option<IpcClient>>,
    pub bridge: RefCell<Option<Arc<WindowBridge>>>,
}

impl fmt::Debug for SharedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedState")
            .field("client_id", &self.client_id.get())
            .field("buffer", &self.buffer.borrow())
            .field("has_composition", &self.composition.borrow().is_some())
            .field("has_ipc_client", &self.ipc_client.borrow().is_some())
            .field("has_bridge", &self.bridge.borrow().is_some())
            .finish()
    }
}

impl TextService {
    pub fn new() -> Self {
        Self {
            thread_mgr: RefCell::new(None),
            client_id: std::cell::Cell::new(0),
            state: Rc::new(SharedState {
                client_id: std::cell::Cell::new(0),
                composition: RefCell::new(None),
                buffer: RefCell::new(String::new()),
                ipc_client: RefCell::new(None),
                bridge: RefCell::new(None),
            }),
        }
    }
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> Result<()> {
        dbg(&format!("[tsf] [STEP 1] Activate called! client_id (tid) = {}", tid));
        let thread_mgr = ptim.cloned().ok_or(windows::core::Error::from(
            windows::Win32::Foundation::E_INVALIDARG,
        ))?;

        // 把自己注册为 KeyEventSink,这样才能收到按键消息
        let keystroke_mgr: ITfKeystrokeMgr = thread_mgr.cast()?;

        unsafe {
            let sink: ITfKeyEventSink = self.cast()?;
            // 1. 无脑尝试清理一次遗留的 tid 绑定（忽略错误）
            let _ = keystroke_mgr.UnadviseKeyEventSink(tid);

            // 2. 仅进行一次Advise 注册，并捕获结果
            match keystroke_mgr.AdviseKeyEventSink(tid, &sink, TRUE) {
                Ok(_) => dbg("[tsf] [STEP 2] AdviseKeyEventSink Succeeded!"),
                Err(e) => {
                    dbg(&format!("[tsf] [STEP 2] AdviseKeyEventSink Failed! Error: {:?}", e));
                    return Err(e);
                }
            }
        }

        // 3. 保存内部状态
        self.client_id.set(tid);
        self.state.client_id.set(tid);

        if let Ok(bridge) = WindowBridge::new(self.state.clone()) {
            let bridge_arc = Arc::new(bridge);
            
            // 2. 启动后台 IPC 管道客户端
            let ipc = IpcClient::start(bridge_arc.clone());

            // 3. 保存到 SharedState 中
            *self.state.bridge.borrow_mut() = Some(bridge_arc);
            *self.state.ipc_client.borrow_mut() = Some(ipc);
        }

        *self.thread_mgr.borrow_mut() = Some(thread_mgr);
        
        dbg("[tsf] Activate succeeded.");
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        dbg("[tsf] Deactivate called!");

        if let Some(thread_mgr) = self.thread_mgr.borrow_mut().take() {
            if let Ok(keystroke_mgr) = thread_mgr.cast::<ITfKeystrokeMgr>() {
                unsafe {
                    match keystroke_mgr.UnadviseKeyEventSink(self.client_id.get()) {
                        Ok(_) => dbg("[tsf] UnadviseKeyEventSink Succeeded!"),
                        Err(e) => dbg(&format!("[tsf] UnadviseKeyEventSink Failed (Ignored): {:?}", e)),
                    }
                }
            } else {
                dbg("Cast to ITfKeystrokeMgr failed in Deactivate");
            }
        }

        self.client_id.set(0);

        self.state.buffer.borrow_mut().clear();
        *self.state.composition.borrow_mut() = None;

        dbg("[tsf] Deactivate complete.");
        Ok(())
    }
}

impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, _fforeground: BOOL) -> Result<()> { dbg(&format!("OnSetFocus called: foreground = {:?}", _fforeground)); Ok(()) }

    fn OnTestKeyDown(&self, _pic: Option<&windows::Win32::UI::TextServices::ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        // 这里判断这个按键是否是你想拦截的(字母、数字、退格等)
        // 返回 TRUE 表示"我要吃掉这个按键",系统才会接着调用 OnKeyDown
        let vk = wparam.0 as u32;
        let interesting = is_interesting_key(vk);
        dbg(&format!(
            "[tsf] [STEP 3] OnTestKeyDown: VK = 0x{:02X}, Intercepted = {}",
            vk, interesting
        ));
        Ok(interesting.into())
    }

    fn OnKeyDown(
        &self,
        pic: Option<&ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        let Some(context) = pic else { return Ok(FALSE) };
        let vk = wparam.0 as u32;

        dbg(&format!("[tsf] [STEP 4] OnKeyDown triggered for VK = 0x{:02X}", vk));

        let action = if vk == 0x08 {
            KeyAction::Backspace
        } else if vk == 0x20 || vk == 0x0D {
            KeyAction::Commit
        } else if (0x41..=0x5A).contains(&vk) {
            KeyAction::Letter(vk_to_char(vk))
        } else {
            return Ok(FALSE);
        };

        let session: ITfEditSession = KeyEditSession {
            state: self.state.clone(),
            context: context.clone(),
            action,
        }
        .into();

        // 卡点 5: RequestEditSession 的返回值
        unsafe {
            dbg("[tsf] Calling RequestEditSession...");
            let res = context.RequestEditSession(
                self.client_id.get(),
                &session,
                TF_ES_SYNC | TF_ES_READWRITE,
            );

            match res {
                Ok(_) => dbg("[tsf] [STEP 5] RequestEditSession returned Ok."),
                Err(e) => dbg(&format!("[tsf] [STEP 5] RequestEditSession failed: {:?}", e)),
            }
        }

        Ok(TRUE)
    }

    fn OnTestKeyUp(&self, _pic: Option<&windows::Win32::UI::TextServices::ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }
    fn OnKeyUp(&self, _pic: Option<&windows::Win32::UI::TextServices::ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }
    fn OnPreservedKey(&self, _pic: Option<&windows::Win32::UI::TextServices::ITfContext>, _rguid: *const windows::core::GUID) -> Result<BOOL> {
        Ok(FALSE)
    }
}

fn is_interesting_key(vk: u32) -> bool {
    // A-Z、退格、空格等——具体范围你可以按需扩展
    (0x41..=0x5A).contains(&vk) || vk == 0x08 || vk == 0x20
}

fn vk_to_char(vk: u32) -> char {
    // A-Z 的虚拟键码正好对应大写 ASCII;这里先统一转小写
    (vk as u8 as char).to_ascii_lowercase()
}