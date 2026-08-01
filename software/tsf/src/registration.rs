// crates/tsf-service/src/registration.rs
use windows::core::{Result, w, GUID, PCWSTR, Interface};
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::{
    RegCreateKeyExW, RegSetValueExW, RegDeleteTreeW, RegCloseKey,
    HKEY, HKEY_CLASSES_ROOT, KEY_WRITE, REG_SZ, REG_OPTION_NON_VOLATILE,
};
use windows::Win32::UI::TextServices::{
    ITfInputProcessorProfiles, ITfCategoryMgr,
    CLSID_TF_InputProcessorProfiles, CLSID_TF_CategoryMgr,
    GUID_TFCAT_TIP_KEYBOARD,
};

use crate::guids::{CLSID_TEXT_SERVICE, GUID_PROFILE};
use crate::dll_module;

const LANGID_EN_US: u16 = 0x0409;
const SERVICE_DESC: &str = "My English IME";

fn clsid_to_reg_key(clsid: &GUID) -> String {
    format!("CLSID\\{{{:?}}}", clsid) // 见下方注意事项
}

/// 获取本 DLL 自身的完整路径,写进 InprocServer32
fn dll_path() -> Result<Vec<u16>> {
    let mut buf = vec![0u16; MAX_PATH as usize];
    let len = unsafe { GetModuleFileNameW(dll_module(), &mut buf) };
    buf.truncate(len as usize);
    buf.push(0);
    Ok(buf)
}

fn set_reg_sz(root: HKEY, subkey: &str, value_name: PCWSTR, data: &[u16]) -> Result<()> {
    unsafe {
        let mut hkey = HKEY::default();
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();

        RegCreateKeyExW(
            root,
            PCWSTR(subkey_w.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
        .ok()?;

        let bytes = std::slice::from_raw_parts(
            data.as_ptr() as *const u8,
            data.len() * 2,
        );
        RegSetValueExW(hkey, value_name, 0, REG_SZ, Some(bytes)).ok()?;
        RegCloseKey(hkey).ok()?;
    }
    Ok(())
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 写 HKCR\CLSID\{...}\InprocServer32
fn register_clsid() -> Result<()> {
    let clsid_key = clsid_to_reg_key(&CLSID_TEXT_SERVICE);

    // (默认值) = 友好名称
    set_reg_sz(HKEY_CLASSES_ROOT, &clsid_key, PCWSTR::null(), &to_wide(SERVICE_DESC))?;

    // InprocServer32 (默认值) = dll 路径
    let inproc_key = format!("{clsid_key}\\InprocServer32");
    let path = dll_path()?;
    set_reg_sz(HKEY_CLASSES_ROOT, &inproc_key, PCWSTR::null(), &path)?;

    // ThreadingModel = Apartment
    set_reg_sz(
        HKEY_CLASSES_ROOT,
        &inproc_key,
        w!("ThreadingModel"),
        &to_wide("Apartment"),
    )?;

    Ok(())
}

fn unregister_clsid() -> Result<()> {
    let clsid_key = to_wide(&format!("CLSID\\{{{:?}}}", CLSID_TEXT_SERVICE));
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(clsid_key.as_ptr()));
    }
    Ok(())
}

pub fn register() -> Result<()> {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };

    let result = (|| -> Result<()> {
        register_clsid()?;

        unsafe {
            // 1. 注册 category:告诉 TSF 这是一个键盘类输入法
            let category_mgr: ITfCategoryMgr =
                CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
            category_mgr.RegisterCategory(
                &CLSID_TEXT_SERVICE,
                &GUID_TFCAT_TIP_KEYBOARD,
                &CLSID_TEXT_SERVICE,
            )?;

            // 2. 注册语言 profile:告诉 TSF 这个输入法用于 en-US,
            //    并关联一个 GUID_PROFILE 供语言栏/设置界面识别
            let profiles: ITfInputProcessorProfiles =
                CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;

            profiles.Register(&CLSID_TEXT_SERVICE)?;

            let desc = to_wide(SERVICE_DESC);
            let icon_path = dll_path()?;

            profiles.AddLanguageProfile(
                &CLSID_TEXT_SERVICE,
                LANGID_EN_US,
                &GUID_PROFILE,
                &desc,
                &icon_path,
                0,
            )?;
        }

        Ok(())
    })();

    unsafe { CoUninitialize() };
    result
}

pub fn unregister() -> Result<()> {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };

    let result = (|| -> Result<()> {
        unsafe {
            let profiles: ITfInputProcessorProfiles =
                CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
            profiles.Unregister(&CLSID_TEXT_SERVICE)?;

            let category_mgr: ITfCategoryMgr =
                CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
            category_mgr.UnregisterCategory(
                &CLSID_TEXT_SERVICE,
                &GUID_TFCAT_TIP_KEYBOARD,
                &CLSID_TEXT_SERVICE,
            )?;
        }

        unregister_clsid()
    })();

    unsafe { CoUninitialize() };
    result
}