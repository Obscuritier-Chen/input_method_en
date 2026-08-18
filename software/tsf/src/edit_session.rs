use std::ptr;
use std::mem::ManuallyDrop;

// crates/tsf-service/src/edit_session.rs
use windows::core::{implement, Result, Interface};
use windows::Win32::UI::TextServices::{
    INSERT_TEXT_AT_SELECTION_FLAGS, ITfComposition, ITfCompositionSink, ITfContext, ITfContextComposition, ITfContextView, ITfEditSession, ITfEditSession_Impl, ITfInsertAtSelection, ITfRange, TF_AE_NONE, TF_ANCHOR_END, TF_ANCHOR_START, TF_IAS_QUERYONLY, TF_SELECTION, TF_SELECTIONSTYLE, TF_ST_CORRECTION,
};

use windows::Win32::Foundation::{BOOL, RECT};

use ime_protocol::ClientRequest;

use crate::text_service::SharedState;

use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
use windows::core::PCWSTR;

fn dbg(msg: &str) {
    let wide: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { OutputDebugStringW(PCWSTR(wide.as_ptr())) };
}

pub enum KeyAction {
    Letter(char),
    Backspace,
    Commit(String),
    Tauricommit,
}

#[implement(ITfEditSession)]
pub struct KeyEditSession {
    pub state: std::rc::Rc<SharedState>,
    pub context: ITfContext,
    pub action: KeyAction,
}

/// 表示屏幕物理像素坐标的矩形区域
#[derive(Debug, Clone, Copy, Default)]
pub struct CursorRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ITfEditSession_Impl for KeyEditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        //dbg("[tsf] doeditsession called");

        // 1. 显式捕获 match 结果，避免 ? 隐式提前返回
        let res = match &self.action {
            KeyAction::Letter(ch) => self.insert_char(ec, *ch),
            KeyAction::Backspace => self.remove_last_char(ec),
            KeyAction::Commit(text) => self.commit(ec, text),
            KeyAction::Tauricommit=>self.tauri_commit(ec),
        };

        if let Err(e) = res {
            dbg(&format!("[tsf] Action executed failed with error: {:?}", e));
            return Err(e);
        }

        // 2. 获取缓冲区文本并强行打印状态
        let buffer = self.state.buffer.borrow().clone();
        dbg(&format!(
            "[tsf] UpdateContext: buffer='{}', composition_active={}",
            buffer,
            self.state.composition.borrow().is_some()
        ));
        let session_id = self.state.client_id.get();
        //dbg(&format!("[tsf] Current buffer: '{}', len: {}", buffer, buffer.len()));

        // 3. 执行 IPC 发送
        if !buffer.is_empty() {
            if let Some(ipc) = self.state.ipc_client.borrow().as_ref() {
                let cursor_rect = self.get_cursor_rect(ec).ok().flatten().map(|r| ime_protocol::CursorRect {
                    left: r.left, top: r.top, right: r.right, bottom: r.bottom, 
                });

                let prefix= self.get_prefix_context(ec)?;

                let req = ClientRequest::UpdateContext {
                    session_id,
                    prefix,
                    buffer: buffer.clone(),
                    cursor_rect,
                };

                dbg(&format!("[tsf] sending UpdateContext: {} to tauri", buffer));
                ipc.send(req);
            } else {
                dbg("[tsf] ipc_client unconnected (is None)!");
            }
        }

        Ok(())
    }
}

fn is_terminating_punct(ch: char) -> bool {
    match ch {
        // 常见的句子/子句截止标点及换行、括号等
        '.' | '!' | '?' | ';' | ':' | ',' | '\n' | '\r' | '"' | '“' | '”' | '(' | ')' | '[' | ']' => true,
        _ => false,
    }
}

fn parse_prefix(raw_text: &str) -> String {
    let char_indices: Vec<(usize, char)> = raw_text.char_indices().collect();
    if char_indices.is_empty() {
        return String::new();
    }

    let mut start_idx = 0;
    let mut word_count = 0;
    let mut in_word = false;

    // 从右往左（从光标紧邻的字符开始向左）逆向遍历
    for &(idx, ch) in char_indices.iter().rev() {
        // 规则 1：遇到截止性标点，立即停止（截取位置在标点之后）
        if is_terminating_punct(ch) {
            start_idx = idx + ch.len_utf8();
            break;
        }

        // 规则 3：统计单词数（以空格/空白区分单词）
        if ch.is_whitespace() {
            if in_word {
                in_word = false;
            }
        } else {
            if !in_word {
                in_word = true;
                word_count += 1;
                // 达到 16 个单词，立即截止
                if word_count > 16 {
                    start_idx = idx + ch.len_utf8();
                    break;
                }
            }
        }
    }

    // 规则 2（达到开头）会在循环自然结束时自动生效（start_idx 保持为 0）
    // 截取并去除最左侧可能残留的空白
    raw_text[start_idx..].trim_start().to_string()
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
        let sink_borrow = self.state.composition_sink.borrow();
        let Some(sink) = sink_borrow.as_ref() else {
            //dbg("[tsf] composition_sink is None!");
            return Err(windows::core::Error::from_hresult(windows::core::HRESULT(-2147024809)));
        };

        let composition = unsafe { context_comp.StartComposition(ec, &range, sink)? };

        *self.state.composition.borrow_mut() = Some(composition.clone());
        Ok(composition)
    }

    fn insert_char(&self, ec: u32, ch: char) -> Result<()> {
        // 1. 更新内存状态 buffer
        self.state.buffer.borrow_mut().push(ch);
        let buffer = self.state.buffer.borrow().clone();
        //dbg(&format!("[tsf] insert_char: buffer is now '{}'", buffer));

        // 2. 获取/创建 Composition 并全量更新 TSF 预输入文本
        let composition = self.ensure_composition(ec)?;
        //dbg("[tsf] Calling composition.GetRange...");
        let range = unsafe { composition.GetRange()? };

        let utf16: Vec<u16> = buffer.encode_utf16().collect();

        unsafe {
            //dbg(&format!("[tsf] Calling range.SetText with utf16 len {}...", utf16.len()));

            range.SetText(ec, 0, &utf16)?;
            range.Collapse(ec, TF_ANCHOR_END)?;
            

            //dbg("[tsf] Calling context.SetSelection...");
            let selection = TF_SELECTION {
                range: ManuallyDrop::new(Some(range.clone())),
                style: TF_SELECTIONSTYLE { ase: TF_AE_NONE, fInterimChar: false.into() },
            };
            self.context.SetSelection(ec, &[selection])?;
        }

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

    fn commit(
        &self,
        ec: u32,
        text: &str,
    ) -> Result<()> {
        let composition = {
            self.state
                .composition
                .borrow()
                .clone()
        };

        let Some(composition) = composition else {
            dbg( "[tsf] Commit requested but no active composition");
            return Ok(());
        };

        let range = unsafe { composition.GetRange()? };

        let utf16: Vec<u16> =
            text.encode_utf16().collect();

        unsafe {
            range.SetText(
                ec,
                0,
                &utf16,
            )?;

            // 2. 把 range 折叠到替换文本末尾
            range.Collapse(
                ec,
                TF_ANCHOR_END,
            )?;

            // 3. 明确设置当前 selection/caret
            let selection = TF_SELECTION {
                range: ManuallyDrop::new(
                    Some(range.clone()),
                ),
                style: TF_SELECTIONSTYLE {
                    ase: TF_AE_NONE,
                    fInterimChar: false.into(),
                },
            };

            self.context.SetSelection(
                ec,
                &[selection],
            )?;

                composition.EndComposition(ec)?;
        }

        dbg(&format!("[tsf] Composition committed: '{}'",text));

        self.state
            .buffer
            .borrow_mut()
            .clear();

        *self.state
            .composition
            .borrow_mut() = None;

        Ok(())
    }

    fn tauri_commit(&self, ec: u32) -> Result<()> {
        if let Some(composition) =
            self.state.composition.borrow_mut().take()
        {
            unsafe {
                composition.EndComposition(ec)?;
            }
        }

        self.state.buffer.borrow_mut().clear();

        Ok(())
    }

    pub fn get_prefix_context(&self, ec: u32) -> Result<String> {
        // 1. 获取当前 Composition 起点，如果没有开启 Composition，则拿当前选区起点
        let range: ITfRange = if let Some(comp) = self.state.composition.borrow().as_ref() {
            unsafe { comp.GetRange()? }
        } else {
            let insert_sel: ITfInsertAtSelection = self.context.cast()?;
            unsafe { insert_sel.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])? }
        };

        // 2. 克隆 Range 并坍缩到起始点（光标左侧位置）
        let prefix_range = unsafe { range.Clone()? };
        unsafe {
            prefix_range.Collapse(ec, TF_ANCHOR_START)?;
        }

        // 3. 向左移动起点 (ShiftStart -200 code units，足以包含 16 个英文单词)
        let mut shifted = 0;
        unsafe {
            prefix_range.ShiftStart(ec, -200, &mut shifted, ptr::null())?;
        }

        // 满足【规则 2】：如果是文档开头，shifted 为 0，直接返回空串
        if shifted == 0 {
            return Ok(String::new());
        }

        // 4. 从扩展后的 Range 中读取文本
        let mut buf = vec![0u16; 200];
        let mut fetched = 0;
        unsafe {
            prefix_range.GetText(ec, 0, &mut buf, &mut fetched)?;
        }

        let raw_text = String::from_utf16_lossy(&buf[..fetched as usize]);

        // 5. 应用 3 大规则过滤并返回最终的 prefix
        Ok(parse_prefix(&raw_text))
    }
    pub fn get_cursor_rect(&self, ec: u32) -> Result<Option<CursorRect>> {
        // 1. 从 context 获取当前的活动视图 ITfContextView
        let view: ITfContextView = match unsafe { self.context.GetActiveView() } {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };

        // 2. 获取当前的 Range（优先取 Composition，无 Composition 则取当前光标选区）
        let range: ITfRange = if let Some(comp) = self.state.composition.borrow().as_ref() {
            unsafe { comp.GetRange()? }
        } else {
            let insert_sel: ITfInsertAtSelection = self.context.cast()?;
            unsafe { insert_sel.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])? }
        };

        // 3. 将 Range 坍缩到末尾（即光标插入点），获取光标处的精确定位
        let cursor_range = unsafe { range.Clone()? };
        unsafe {
            cursor_range.Collapse(ec, TF_ANCHOR_END)?;
        }

        // 4. 调用 GetTextExt 获取该 Range 在屏幕上的包围矩形
        let mut rect = RECT::default();
        let mut clipped = BOOL::default();

        unsafe {
            // GetTextExt 可能因目标文本控件暂未渲染或不支持而返回 Err/0
            if view.GetTextExt(ec, &cursor_range, &mut rect, &mut clipped).is_ok() {
                // 部分异常情况（如非标准控件）可能返回全 0 坐标，需要过滤
                if rect.left == 0 && rect.top == 0 && rect.right == 0 && rect.bottom == 0 {
                    return Ok(None);
                }

                Ok(Some(CursorRect {
                    left: rect.left,
                    top: rect.top,
                    right: rect.right,
                    bottom: rect.bottom,
                }))
            } else {
                Ok(None)
            }
        }
    }
}