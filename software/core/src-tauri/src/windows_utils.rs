#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW,
    SetWindowLongPtrW,
    GWL_EXSTYLE,
    WS_EX_NOACTIVATE,
    SW_SHOWNA,
    ShowWindow,
    WM_MOUSEACTIVATE,
    MA_NOACTIVATE,
    SetWindowPos,
    HWND_TOPMOST,
    SWP_NOMOVE,
    SWP_NOSIZE,
    SWP_NOACTIVATE,
};

#[cfg(target_os = "windows")]
pub fn configure_no_activate(hwnd: HWND) {
    unsafe {
        // ------------------------------------------------------------
        // 1. 增加 WS_EX_NOACTIVATE
        // ------------------------------------------------------------

        let ex_style =
            GetWindowLongPtrW(hwnd, GWL_EXSTYLE);

        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            ex_style | WS_EX_NOACTIVATE.0 as isize,
        );

        // ------------------------------------------------------------
        // 2. 保持 TOPMOST，同时绝不激活
        // ------------------------------------------------------------

        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE
                | SWP_NOSIZE
                | SWP_NOACTIVATE,
        );
    }
}