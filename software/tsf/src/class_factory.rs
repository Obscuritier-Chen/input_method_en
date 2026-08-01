use windows::core::{implement, Result, GUID, IUnknown, Interface};
use windows::Win32::Foundation::{BOOL, CLASS_E_NOAGGREGATION, E_POINTER, S_OK};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};

use crate::text_service::TextService;

#[implement(IClassFactory)]
pub struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        unsafe {
            if ppvobject.is_null() {
                return Err(E_POINTER.into());
            }
            *ppvobject = std::ptr::null_mut();

            // TSF 不需要聚合,直接拒绝
            if punkouter.is_some() {
                return Err(CLASS_E_NOAGGREGATION.into());
            }

            let service: IUnknown = TextService::new().into();
            service.query(riid, ppvobject).ok()
        }
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}

/// DllGetClassObject 里调用:构造一个 ClassFactory,再 QueryInterface 到调用方要的接口
/// (通常就是 IClassFactory 本身)
pub unsafe fn create_class_factory(
    riid: *const GUID,
    ppv: *mut *mut core::ffi::c_void,
) -> windows::core::HRESULT {
    if ppv.is_null() {
        return E_POINTER;
    }
    *ppv = std::ptr::null_mut();

    let factory: IUnknown = ClassFactory.into();
    match factory.query(riid, ppv).ok() {
        Ok(_) => S_OK,
        Err(e) => e.code(),
    }
}