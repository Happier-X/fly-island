//! Dragging shelf files out to Explorer, chat windows and other OLE drop targets.

use std::{ffi::OsString, iter::once, mem::size_of, os::windows::ffi::OsStrExt, path::Path, ptr};

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{GlobalFree, HGLOBAL, HWND},
        System::{
            Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL},
            DataExchange::RegisterClipboardFormatW,
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
            Ole::{IDropSource, OleInitialize, OleUninitialize, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE},
        },
        UI::Shell::{
            BHID_DataObject, Common::ITEMIDLIST, ILCreateFromPathW, ILFree, IShellItemArray,
            SHCreateShellItemArrayFromIDLists, SHDoDragDrop,
        },
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragOutcome {
    Copied,
    Moved,
    Linked,
    None,
}

struct OleGuard;

impl OleGuard {
    fn new() -> Result<Self, String> {
        // SAFETY: no pointers; a success including S_FALSE is balanced in Drop.
        unsafe { OleInitialize(None) }.map_err(|error| format!("初始化 OLE 失败：{error}"))?;
        Ok(Self)
    }
}

impl Drop for OleGuard {
    fn drop(&mut self) {
        // SAFETY: pairs OleInitialize in new.
        unsafe { OleUninitialize() };
    }
}

struct ItemIdList(*mut ITEMIDLIST);

impl ItemIdList {
    fn from_path(path: &Path) -> Result<Self, String> {
        let text = shell_path(path);
        let wide: Vec<u16> = text.encode_wide().chain(once(0)).collect();
        // SAFETY: wide is NUL-terminated and outlives the call.
        let list = unsafe { ILCreateFromPathW(PCWSTR(wide.as_ptr())) };
        if list.is_null() {
            return Err(format!("文件不存在：{}", path.display()));
        }
        Ok(Self(list))
    }
}

impl Drop for ItemIdList {
    fn drop(&mut self) {
        // SAFETY: the pointer came from ILCreateFromPathW and is freed once.
        unsafe { ILFree(Some(self.0)) };
    }
}

fn shell_path(path: &Path) -> OsString {
    let text = path.as_os_str().to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}").into();
    }
    match text.strip_prefix(r"\\?\") {
        Some(rest) => rest.into(),
        None => path.as_os_str().to_owned(),
    }
}

/// Starts a modal drag from the AppView window. Must run on the window's UI thread.
pub fn drag_files(window: HWND, paths: &[String]) -> Result<DragOutcome, String> {
    if paths.is_empty() {
        return Ok(DragOutcome::None);
    }
    let _ole = OleGuard::new()?;
    let lists = paths
        .iter()
        .map(|path| ItemIdList::from_path(Path::new(path)))
        .collect::<Result<Vec<_>, _>>()?;
    let pointers: Vec<*const ITEMIDLIST> = lists.iter().map(|list| list.0.cast_const()).collect();
    // SAFETY: pointers are live ID lists owned by lists; the array copies them.
    let items: IShellItemArray = unsafe { SHCreateShellItemArrayFromIDLists(&pointers) }
        .map_err(|error| format!("准备拖出文件失败：{error}"))?;
    // SAFETY: BHID_DataObject is static and no bind context is needed.
    let data: IDataObject = unsafe { items.BindToHandler(None, &BHID_DataObject) }
        .map_err(|error| format!("准备拖出文件失败：{error}"))?;
    set_preferred_effect(&data, DROPEFFECT_COPY)?;

    let allowed = DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK;
    // SAFETY: data is live, the shell owns the drop source, and window is live on this thread.
    let effect = unsafe { SHDoDragDrop(Some(window), &data, None::<&IDropSource>, allowed) }
        .map_err(|error| format!("拖出失败：{error}"))?;
    Ok(outcome(effect))
}

fn outcome(effect: DROPEFFECT) -> DragOutcome {
    if effect.0 & DROPEFFECT_MOVE.0 != 0 {
        DragOutcome::Moved
    } else if effect.0 & DROPEFFECT_COPY.0 != 0 {
        DragOutcome::Copied
    } else if effect.0 & DROPEFFECT_LINK.0 != 0 {
        DragOutcome::Linked
    } else {
        DragOutcome::None
    }
}

fn set_preferred_effect(data: &IDataObject, effect: DROPEFFECT) -> Result<(), String> {
    // SAFETY: the name is static NUL-terminated UTF-16.
    let format = unsafe { RegisterClipboardFormatW(w!("Preferred DropEffect")) };
    if format == 0 {
        return Err("注册拖放格式失败。".to_string());
    }
    // SAFETY: one movable u32 block; ownership goes to the data object or is freed below.
    let memory: HGLOBAL = unsafe { GlobalAlloc(GMEM_MOVEABLE, size_of::<u32>()) }
        .map_err(|error| format!("分配拖放数据失败：{error}"))?;
    // SAFETY: memory is a live movable block of four bytes.
    unsafe {
        let pointer = GlobalLock(memory).cast::<u32>();
        if pointer.is_null() {
            let _ = GlobalFree(Some(memory));
            return Err("写入拖放数据失败。".to_string());
        }
        pointer.write_unaligned(effect.0);
        let _ = GlobalUnlock(memory);
    }
    let format_etc = FORMATETC {
        cfFormat: format as u16,
        ptd: ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    let medium = STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 { hGlobal: memory },
        pUnkForRelease: std::mem::ManuallyDrop::new(None),
    };
    // SAFETY: both structs are valid; SetData with fRelease takes ownership on success.
    if let Err(error) = unsafe { data.SetData(&format_etc, &medium, true) } {
        // SAFETY: SetData failed, so memory is still ours.
        let _ = unsafe { GlobalFree(Some(memory)) };
        return Err(format!("设置拖放数据失败：{error}"));
    }
    Ok(())
}
