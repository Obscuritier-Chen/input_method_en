// crates/tsf-service/src/lib.rs
mod guids;
mod text_service;
mod class_factory;
mod registration;
mod edit_session;
mod window_bridge;
mod commit_session;
mod ipc_client;

use windows::core::{HRESULT, GUID};
use windows::Win32::Foundation::{HMODULE, S_OK, CLASS_E_CLASSNOTAVAILABLE, S_FALSE};
use windows::Win32::System::Com::IClassFactory;
use std::sync::atomic::{AtomicIsize, Ordering};
use windows::Win32::Foundation::HINSTANCE;

static DLL_INSTANCE: AtomicIsize = AtomicIsize::new(0);

use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
use windows::core::PCWSTR;

fn dbg(msg: &str) {
    let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { OutputDebugStringW(PCWSTR(wide.as_ptr())) };
}

#[no_mangle]
extern "system" fn DllMain(hinstance: HINSTANCE, _reason: u32, _reserved: *mut core::ffi::c_void) -> i32 {
    //dbg("DllMain called");
    DLL_INSTANCE.store(hinstance.0 as isize, Ordering::SeqCst);
    1 // TRUE
}

pub(crate) fn dll_module() -> HINSTANCE {
    HINSTANCE(DLL_INSTANCE.load(Ordering::SeqCst) as _)
}

#[no_mangle]
extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut core::ffi::c_void,
) -> HRESULT {
    unsafe {
        if *rclsid != guids::CLSID_TEXT_SERVICE {
            return CLASS_E_CLASSNOTAVAILABLE;
        }
        class_factory::create_class_factory(riid, ppv)
    }
}

#[no_mangle]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    // 简化处理,先总是返回 S_FALSE(表示"先别卸载我")
    S_FALSE
}

#[no_mangle]
extern "system" fn DllRegisterServer() -> HRESULT {
    match registration::register() {
        Ok(_) => S_OK,
        Err(e) => e.code(),
    }
}

#[no_mangle]
extern "system" fn DllUnregisterServer() -> HRESULT {
    match registration::unregister() {
        Ok(_) => S_OK,
        Err(e) => e.code(),
    }
}