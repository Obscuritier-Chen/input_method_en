use windows::core::PCWSTR;
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};

pub struct SecurityAttributesGuard {
    sa: SECURITY_ATTRIBUTES,
    sd: PSECURITY_DESCRIPTOR,
}

impl SecurityAttributesGuard {
    /// 传入 SDDL 字符串创建 SecurityAttributes 包装
    pub fn new_sddl(sddl: &str) -> windows::core::Result<Self> {
        let wide_sddl: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();
        let mut sd = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(wide_sddl.as_ptr()),
                SDDL_REVISION_1,
                &mut sd,
                None,
            )?;
        }

        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd.0,
            bInheritHandle: false.into(),
        };

        Ok(Self { sa, sd })
    }

    /// 获取传给 Windows API 的裸指针
    pub fn as_raw_ptr(&mut self) -> *mut std::ffi::c_void {
        &mut self.sa as *mut _ as *mut _
    }
}

impl Drop for SecurityAttributesGuard {
    fn drop(&mut self) {
        if !self.sd.0.is_null() {
            unsafe {
                let _ = LocalFree(HLOCAL(self.sd.0));
            }
        }
    }
}