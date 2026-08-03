// crates/tsf-service/src/edit_session.rs
use windows::core::{implement, Result, Interface};
use windows::Win32::UI::TextServices::{
    ITfEditSession, ITfEditSession_Impl, ITfContext, ITfContextComposition,
    ITfComposition, ITfInsertAtSelection, ITfRange,
    TF_IAS_QUERYONLY, TF_ANCHOR_END, TF_ST_CORRECTION,
};

use crate::text_service::SharedState;

pub enum KeyAction {
    Letter(char),
    Backspace,
    Commit,
}

#[implement(ITfEditSession)]
pub struct KeyEditSession {
    pub state: std::rc::Rc<SharedState>,
    pub context: ITfContext,
    pub action: KeyAction,
}

impl ITfEditSession_Impl for KeyEditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        match &self.action {
            KeyAction::Letter(ch) => self.insert_char(ec, *ch),
            KeyAction::Backspace => self.remove_last_char(ec),
            KeyAction::Commit => self.commit(ec),
        }
    }
}

impl KeyEditSession_Impl {
    fn ensure_composition(&self, ec: u32) -> Result<ITfComposition> {
        if let Some(comp) = self.state.composition.borrow().as_ref() {
            return Ok(comp.clone());
        }

        // 拿当前选区作为 composition 的起点
        let insert_sel: ITfInsertAtSelection = self.context.cast()?;
        let range: ITfRange = unsafe {
            insert_sel.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?
        };

        let context_comp: ITfContextComposition = self.context.cast()?;
        let composition = unsafe { context_comp.StartComposition(ec, &range, None)? };

        *self.state.composition.borrow_mut() = Some(composition.clone());
        Ok(composition)
    }

    fn insert_char(&self, ec: u32, ch: char) -> Result<()> {
        let composition = self.ensure_composition(ec)?;
        let range = unsafe { composition.GetRange()? };

        // 把光标移到 range 末尾再插入新字符
        unsafe {
            range.Collapse(ec, TF_ANCHOR_END)?;
            let text: Vec<u16> = ch.encode_utf16(&mut [0u16; 2]).to_vec();
            range.SetText(ec, TF_ST_CORRECTION, &text)?;
        }

        self.state.buffer.borrow_mut().push(ch);

        // TODO: 这里之后接命名管道,把 buffer 内容发给 Tauri 主进程请求候选词
        println!("当前输入: {}", self.state.buffer.borrow());

        Ok(())
    }

    fn remove_last_char(&self, ec: u32) -> Result<()> {
        let mut buffer = self.state.buffer.borrow_mut();
        if buffer.is_empty() {
            return Ok(());
        }
        buffer.pop();

        if let Some(composition) = self.state.composition.borrow().as_ref() {
            let range = unsafe { composition.GetRange()? };
            // 简化处理:直接用 buffer 剩余内容整体重写这段 range
            let text: Vec<u16> = buffer.encode_utf16().collect();
            unsafe {
                range.SetText(ec, 0, &text)?;
            }
        }

        Ok(())
    }

    fn commit(&self, ec: u32) -> Result<()> {
        if let Some(composition) = self.state.composition.borrow_mut().take() {
            unsafe {
                composition.EndComposition(ec)?;
            }
        }
        self.state.buffer.borrow_mut().clear();
        Ok(())
    }
}