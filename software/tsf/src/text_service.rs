use std::cell::RefCell;
use std::rc::Rc;

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

#[derive(Debug, Clone)]

#[implement(ITfTextInputProcessor, ITfKeyEventSink)]
pub struct TextService {
    thread_mgr: RefCell<Option<ITfThreadMgr>>,
    client_id: std::cell::Cell<u32>,
    state: Rc<SharedState>,
}

#[derive(Debug)]
pub struct SharedState {
    pub composition: RefCell<Option<ITfComposition>>,
    pub buffer: RefCell<String>,
}

impl TextService {
    pub fn new() -> Self {
        Self {
            thread_mgr: RefCell::new(None),
            client_id: std::cell::Cell::new(0),
            state: Rc::new(SharedState {
                composition: RefCell::new(None),
                buffer: RefCell::new(String::new()),
            }),
        }
    }
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> Result<()> {
        dbg(&format!(">>> [STEP 1] Activate called! client_id (tid) = {}", tid));
        let thread_mgr = ptim.cloned().ok_or(windows::core::Error::from(
            windows::Win32::Foundation::E_INVALIDARG,
        ))?;

        // 把自己注册为 KeyEventSink,这样才能收到按键消息
        let keystroke_mgr: ITfKeystrokeMgr = thread_mgr.cast()?;
        unsafe {
            let sink: ITfKeyEventSink = self.cast()?;
            match keystroke_mgr.AdviseKeyEventSink(tid, &sink, TRUE) {
                Ok(_) => dbg("✅ [STEP 2] AdviseKeyEventSink Succeeded!"),
                Err(e) => dbg(&format!("❌ [STEP 2] AdviseKeyEventSink Failed! Error: {:?}", e)),
            }
        }
        unsafe {
            let sink: ITfKeyEventSink = self.cast()?;
            keystroke_mgr.AdviseKeyEventSink(tid, &sink, TRUE)?;
        }

        self.client_id.set(tid);
        *self.thread_mgr.borrow_mut() = Some(thread_mgr);
        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        if let Some(thread_mgr) = self.thread_mgr.borrow_mut().take() {
            let keystroke_mgr: ITfKeystrokeMgr = thread_mgr.cast()?;
            unsafe {
                keystroke_mgr.UnadviseKeyEventSink(self.client_id.get())?;
            }
        }
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
            "--> [STEP 3] OnTestKeyDown: VK = 0x{:02X}, Intercepted = {}",
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

        dbg(&format!("--> [STEP 4] OnKeyDown triggered for VK = 0x{:02X}", vk));

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
            dbg("--> Calling RequestEditSession...");
            let res = context.RequestEditSession(
                self.client_id.get(),
                &session,
                TF_ES_SYNC | TF_ES_READWRITE,
            );

            match res {
                Ok(_) => dbg("✅ [STEP 5] RequestEditSession returned Ok."),
                Err(e) => dbg(&format!("❌ [STEP 5] RequestEditSession failed: {:?}", e)),
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