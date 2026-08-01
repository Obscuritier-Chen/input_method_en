// crates/tsf-service/src/text_service.rs
use windows::core::{implement, IUnknown, Result, Interface};
use windows::Win32::UI::TextServices::{
    ITfTextInputProcessor, ITfTextInputProcessor_Impl,
    ITfThreadMgr, ITfKeyEventSink, ITfKeyEventSink_Impl,
    ITfKeystrokeMgr,
};
use windows::Win32::Foundation::{BOOL, WPARAM, LPARAM, TRUE, FALSE};

#[implement(ITfTextInputProcessor, ITfKeyEventSink)]
pub struct TextService {
    thread_mgr: std::cell::RefCell<Option<ITfThreadMgr>>,
    client_id: std::cell::Cell<u32>,
}

impl TextService {
    pub fn new() -> Self {
        Self {
            thread_mgr: std::cell::RefCell::new(None),
            client_id: std::cell::Cell::new(0),
        }
    }
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> Result<()> {
        let thread_mgr = ptim.cloned().ok_or(windows::core::Error::from(
            windows::Win32::Foundation::E_INVALIDARG,
        ))?;

        // 把自己注册为 KeyEventSink,这样才能收到按键消息
        let keystroke_mgr: ITfKeystrokeMgr = thread_mgr.cast()?;
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
    fn OnSetFocus(&self, _fforeground: BOOL) -> Result<()> { Ok(()) }

    fn OnTestKeyDown(&self, _pic: Option<&windows::Win32::UI::TextServices::ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        // 这里判断这个按键是否是你想拦截的(字母、数字、退格等)
        // 返回 TRUE 表示"我要吃掉这个按键",系统才会接着调用 OnKeyDown
        Ok(is_interesting_key(wparam.0 as u32).into())
    }

    fn OnKeyDown(&self, pic: Option<&windows::Win32::UI::TextServices::ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        // 真正处理按键:维护 composition string、通过命名管道向 Tauri 主进程请求候选词
        // TODO: 接入 IPC
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