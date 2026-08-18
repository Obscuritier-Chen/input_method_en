use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{PCWSTR, Result, w};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, PostMessageW,
    RegisterClassW, HWND_MESSAGE, MSG, WM_USER, WNDCLASSW,
    CREATESTRUCTW,
    DefWindowProcW,
    GetWindowLongPtrW,
    SetWindowLongPtrW,
    GWLP_USERDATA,
    WM_NCCREATE,
};
use windows::Win32::UI::TextServices::{ITfContext, ITfEditSession, TF_ES_ASYNCDONTCARE, TF_ES_READWRITE, TF_ES_SYNC};

use crate::commit_session::CommitEditSession;
use crate::text_service::SharedState;
use crate::edit_session::{KeyEditSession, KeyAction};

use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;

fn dbg(msg: &str) {
    let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { OutputDebugStringW(PCWSTR(wide.as_ptr())) };
}

pub const WM_IME_COMMIT: u32 = WM_USER + 101;

pub struct WindowBridge {
    hwnd: HWND,
}

impl WindowBridge {
    /// 必须在 TextService::Activate（即主 STA 线程）中创建
    pub fn new(state: Rc<SharedState>) -> Result<Self> {
        let class_name = w!("MyTsfBridgeClass");
        //let class_pcwstr = PCWSTR(class_name.as_ptr());

        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            lpszClassName: class_name,
            ..Default::default()
        };

        unsafe {
            RegisterClassW(&wnd_class);

            // 保存 Context 和 ClientId 到全局或传递指针，这里使用 Box 包装结构传给 WndProc
            let state_ptr = Rc::into_raw(state.clone()) as *const std::ffi::c_void;

            Some(state_ptr);

            let hwnd = CreateWindowExW(
                Default::default(),
                class_name,
                PCWSTR::null(),
                Default::default(),
                0, 0, 0, 0,
                HWND_MESSAGE, // 消息专用隐藏窗口，不显示 UI
                None,
                None,
                Some(state_ptr),
            )?;

            Ok(Self { hwnd })
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// 向该隐藏窗口投递提交候选词消息（可由后台管道线程跨线程安全调用）
    pub fn post_commit(&self, text: String) {
        // 将 String 转换为裸指针投递，传递给 WndProc，避免跨线程内存析构问题
        let ptr = Box::into_raw(Box::new(text));
        unsafe {
            let _ = PostMessageW(self.hwnd, WM_IME_COMMIT, WPARAM(0), LPARAM(ptr as isize));
        }
    }
}

impl Drop for WindowBridge {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe impl Send for WindowBridge {}
unsafe impl Sync for WindowBridge {}

struct BridgeState {
    context: ITfContext,
    client_id: u32,
}

unsafe fn clone_shared_state_from_hwnd(hwnd: HWND) -> Option<Rc<SharedState>> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);

    if ptr == 0 {
        return None;
    }

    let ptr = ptr as *const SharedState;

    Rc::increment_strong_count(ptr);

    Some(Rc::from_raw(ptr))
}

/// 运行在主 STA 线程中的窗口回调函数
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let create_struct = &*(lparam.0 as *const CREATESTRUCTW);

            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                create_struct.lpCreateParams as isize,
            );

            return LRESULT(1);
        }

        WM_IME_COMMIT => {
            if lparam.0 != 0 {
                let text =
                    *Box::from_raw(
                        lparam.0 as *mut String,
                    );

                dbg(&format!("[tsf] WM_IME_COMMIT received: '{}'", text));

                if let Err(e) = trigger_commit(hwnd, text){
                    dbg(&format!(  "[tsf] trigger_commit failed: {:?}",e));
                }
            }

            return LRESULT(0);
        }
        
        _ => {}
    }

    DefWindowProcW(
        hwnd,
        msg,
        wparam,
        lparam,
    )
}

fn trigger_commit(hwnd: HWND, text: String) -> windows_core::Result<()>{
    let state =
    unsafe {
        clone_shared_state_from_hwnd(hwnd)
    }
    .ok_or_else(|| {
        windows::core::Error::from_win32()
    })?;

    let composition = {
        let composition_ref =
            state.composition.borrow();

        composition_ref.clone().ok_or_else(|| {
            windows::core::Error::from_hresult(
                windows::core::HRESULT(
                    -2147024809,
                ),
            )
        })?
    };

    let range = unsafe {
        composition.GetRange()?
    };

    let context = unsafe {
        range.GetContext()?
    };

    dbg(&format!("[tsf] trigger_commit: client_id={}",state.client_id.get()));

    let action = KeyAction::Commit(text);

    let edit_session = KeyEditSession {
        state: state.clone(),
        context: context.clone(),
        action,
    };

    let edit_session: ITfEditSession =
        edit_session.into();

    unsafe {
        let session_result = context.RequestEditSession(
            state.client_id.get(),
            &edit_session,
            TF_ES_SYNC| TF_ES_READWRITE,
        )?;

        dbg(&format!("[tsf] RequestEditSession client_id={}",state.client_id.get()));

        dbg(&format!("[tsf] Commit RequestEditSession result = 0x{:08X}", session_result.0));
    }

    

    Ok(())
}