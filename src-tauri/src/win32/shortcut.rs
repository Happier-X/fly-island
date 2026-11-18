//! Shortcut targets: what a Start Menu `.lnk` points at, shown under its name in the
//! quick launcher. Launching the shortcut itself is left to the shell.

use windows::{
    core::{Interface, PCWSTR},
    Win32::{
        System::Com::{CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ},
        UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH},
    },
};

use super::com::ComApartment;

/// Path a `.lnk` file points at, or `None` when it cannot be read (broken links,
/// links to shell items without a file path). COM is set up for the calling thread.
pub(crate) fn shortcut_target(path: &str) -> Option<String> {
    let _com = ComApartment::multithreaded().ok();
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: ShellLink is a registered in-process class known to the shell, no outer
    // object is passed and the returned interface is owned.
    let link: IShellLinkW =
        unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }.ok()?;
    // SAFETY: IShellLinkW implements IPersistFile; the cast loans the same object.
    let file: IPersistFile = link.cast().ok()?;
    // SAFETY: `wide` is a NUL-terminated UTF-16 path that outlives the call; STGM_READ
    // only reads the shortcut file.
    unsafe { file.Load(PCWSTR(wide.as_ptr()), STGM_READ) }.ok()?;

    let mut buffer = [0u16; 1024];
    // SAFETY: `buffer` is writable for its whole length; no WIN32_FIND_DATAW is
    // requested (null), and SLGP_RAWPATH asks for the stored path unchanged.
    unsafe { link.GetPath(&mut buffer, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32) }.ok()?;
    let length = buffer.iter().position(|unit| *unit == 0).unwrap_or(0);
    (length > 0).then(|| String::from_utf16_lossy(&buffer[..length]))
}

#[cfg(test)]
mod tests {
    use super::shortcut_target;

    #[test]
    fn a_missing_shortcut_has_no_target() {
        assert_eq!(shortcut_target(r"C:\does\not\exist\missing.lnk"), None);
    }
}
