//! Multi-select file picker for the file shelf.

use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{ERROR_CANCELLED, HWND},
        System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_INPROC_SERVER},
        UI::Shell::{
            FileOpenDialog, IFileOpenDialog, IShellItemArray, FOS_ALLOWMULTISELECT,
            FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_PATHMUSTEXIST, SIGDN_FILESYSPATH,
        },
    },
};

/// Shows the system multi-select file dialog. An empty list means cancelled.
pub fn pick_files(owner: HWND) -> Result<Vec<String>, String> {
    // SAFETY: the class and interface are static shell contracts; owner is the live AppView.
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }
            .map_err(|error| format!("打开文件选择器失败：{error}"))?;
    // SAFETY: the dialog is live.
    unsafe {
        dialog
            .SetOptions(
                FOS_ALLOWMULTISELECT
                    | FOS_FILEMUSTEXIST
                    | FOS_PATHMUSTEXIST
                    | FOS_FORCEFILESYSTEM,
            )
            .map_err(|error| format!("设置文件选择器失败：{error}"))?;
    }
    // SAFETY: owner is the live AppView window; cancellation is a normal result.
    if let Err(error) = unsafe { dialog.Show(Some(owner)) } {
        if error.code().0 as u32 == ERROR_CANCELLED.0 {
            return Ok(Vec::new());
        }
        return Err(format!("文件选择失败：{error}"));
    }
    // SAFETY: the dialog is live after Show.
    let results: IShellItemArray = unsafe { dialog.GetResults() }
        .map_err(|error| format!("读取选中文件失败：{error}"))?;
    // SAFETY: GetCount is a plain array query.
    let count = unsafe { results.GetCount() }.map_err(|error| format!("读取选中文件数量失败：{error}"))?;
    let mut paths = Vec::new();
    for index in 0..count {
        // SAFETY: index is below the count just read.
        let item = unsafe { results.GetItemAt(index) }
            .map_err(|error| format!("读取选中文件失败：{error}"))?;
        // SAFETY: the item is live.
        let name: PWSTR = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }
            .map_err(|error| format!("读取文件路径失败：{error}"))?;
        // SAFETY: GetDisplayName returns an allocated NUL-terminated string.
        let text = unsafe { name.to_string() }.map_err(|error| format!("读取文件路径失败：{error}"))?;
        // SAFETY: the shell allocated this string with CoTaskMemAlloc.
        unsafe { CoTaskMemFree(Some(name.0 as *const _)) };
        if !text.is_empty() {
            paths.push(text);
        }
    }
    Ok(paths)
}
