//! The user's known folders (Desktop, Documents, Downloads), which can be moved away
//! from the profile folder, so they are asked from Windows rather than assumed.

use std::path::PathBuf;
use windows::{
    core::GUID,
    Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{
            FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, SHGetKnownFolderPath,
            KF_FLAG_DEFAULT,
        },
    },
};

/// The current user's Downloads folder; `None` if Windows cannot say.
pub(crate) fn downloads() -> Option<PathBuf> {
    known_folder(&FOLDERID_Downloads)
}

/// The current user's Desktop folder; `None` if Windows cannot say.
pub(crate) fn desktop() -> Option<PathBuf> {
    known_folder(&FOLDERID_Desktop)
}

/// The current user's Documents folder; `None` if Windows cannot say.
pub(crate) fn documents() -> Option<PathBuf> {
    known_folder(&FOLDERID_Documents)
}

fn known_folder(id: &GUID) -> Option<PathBuf> {
    // SAFETY: `id` is a valid known-folder id, no token is passed (current user), and
    // the returned buffer is freed below with CoTaskMemFree, as SHGetKnownFolderPath
    // requires, whether or not it reads as a string.
    let path = unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) }.ok()?;
    // SAFETY: on success `path` points to a NUL-terminated UTF-16 string.
    let text = unsafe { path.to_string() };
    // SAFETY: `path` came from SHGetKnownFolderPath and is freed exactly once.
    unsafe { CoTaskMemFree(Some(path.0.cast())) };
    text.ok().filter(|text| !text.is_empty()).map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::{desktop, documents, downloads};

    #[test]
    fn the_known_folders_are_absolute_paths() {
        for (name, folder) in [
            ("Desktop", desktop()),
            ("Documents", documents()),
            ("Downloads", downloads()),
        ] {
            let path = folder.unwrap_or_else(|| panic!("{name}"));
            assert!(path.is_absolute(), "{path:?}");
        }
    }
}
