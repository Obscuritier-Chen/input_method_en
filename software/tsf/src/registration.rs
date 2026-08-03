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
    GUID_TFCAT_TIP_KEYBOARD, GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
};

use crate::guids::{CLSID_TEXT_SERVICE, GUID_PROFILE};
use crate::dll_module;

// 🎯 补全 Immersive App (UWP/Win11设置界面) 支持分类 GUID
// GUID: {48678036-264D-4A23-AE27-2FA77469B46C}
const GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT: GUID = GUID::from_u128(0x48678036_264d_4a23_ae27_2fa77469b46c);

const LANGID_EN_US: u16 = 0x0409;
const SERVICE_DESC: &str = "My English IME";

/// 修复：正确格式化 GUID 为标准的标准注册表路径字符串
fn format_guid(guid: &GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        guid.data1, guid.data2, guid.data3,
        guid.data4[0], guid.data4[1], guid.data4[2], guid.data4[3],
        guid.data4[4], guid.data4[5], guid.data4[6], guid.data4[7],
    )
}

fn clsid_to_reg_key(clsid: &GUID) -> String {
    format!("CLSID\\{}", format_guid(clsid))
}

/// 获取本 DLL 自身的完整路径
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
    let clsid_key = to_wide(&clsid_to_reg_key(&CLSID_TEXT_SERVICE));
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
            let category_mgr: ITfCategoryMgr =
                CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;

            // 1. 注册基础 Category: 声明为键盘输入法
            category_mgr.RegisterCategory(
                &CLSID_TEXT_SERVICE,
                &GUID_TFCAT_TIP_KEYBOARD,
                &CLSID_TEXT_SERVICE,
            )?;

            // 2. 🎯 核心修复：注册 Immersive Support Category
            // 告诉 Windows 设置和 Modern 应用，此输入法可以在全局（包括设置界面）激活，消除灰色“仅桌面”状态
            category_mgr.RegisterCategory(
                &CLSID_TEXT_SERVICE,
                &GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
                &CLSID_TEXT_SERVICE,
            )?;

            // 3. 注册 Display Attribute Provider（推荐）
            category_mgr.RegisterCategory(
                &CLSID_TEXT_SERVICE,
                &GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
                &CLSID_TEXT_SERVICE,
            )?;

            // 4. 注册语言 profile
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

            // 卸载所有 Category
            category_mgr.UnregisterCategory(
                &CLSID_TEXT_SERVICE,
                &GUID_TFCAT_TIP_KEYBOARD,
                &CLSID_TEXT_SERVICE,
            )?;

            category_mgr.UnregisterCategory(
                &CLSID_TEXT_SERVICE,
                &GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
                &CLSID_TEXT_SERVICE,
            )?;

            category_mgr.UnregisterCategory(
                &CLSID_TEXT_SERVICE,
                &GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
                &CLSID_TEXT_SERVICE,
            )?;
        }

        unregister_clsid()
    })();

    unsafe { CoUninitialize() };
    result
}