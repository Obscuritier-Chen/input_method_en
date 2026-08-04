// crates/tsf-service/src/commit_session.rs
use windows::core::{implement, Result};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfContext, ITfEditSession, ITfEditSession_Impl, ITfRange, TF_ST_CORRECTION,
};

#[implement(ITfEditSession)]
pub struct CommitEditSession {
    pub context: ITfContext,
    pub composition: Option<ITfComposition>,
    pub text: String,
}

impl ITfEditSession_Impl for CommitEditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        let text_u16: Vec<u16> = self.text.encode_utf16().collect();

        if let Some(comp) = &self.composition {
            unsafe {
                let range: ITfRange = comp.GetRange()?;
                // 用选中的词替换整个 Composition Range
                range.SetText(ec, TF_ST_CORRECTION, &text_u16)?;
                // 结束组合态
                comp.EndComposition(ec)?;
            }
        }
        Ok(())
    }
}