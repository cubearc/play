//! Only OS-resolved paths needed for AppContainer startup; no inherited secrets.
use std::{io, ptr};
use windows_sys::Win32::{
    System::{Com::CoTaskMemFree, SystemInformation::GetWindowsDirectoryW},
    UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
};
struct Folder(*mut u16);
impl Drop for Folder {
    fn drop(&mut self) {
        // SAFETY: SHGetKnownFolderPath transfers this allocation, including on failure.
        unsafe {
            CoTaskMemFree(self.0.cast());
        }
    }
}
fn local_app_data() -> io::Result<Vec<u16>> {
    let mut folder = Folder(ptr::null_mut());
    // SAFETY: documented folder ID, current user, live owning output pointer.
    let code =
        unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, ptr::null_mut(), &mut folder.0) };
    if code < 0 || folder.0.is_null() {
        return Err(io::Error::other(
            "OS local application data path unavailable",
        ));
    }
    let mut result = Vec::new();
    for index in 0..32768 {
        // SAFETY: success guarantees a live nul-terminated UTF-16 string until Folder drops.
        let ch = unsafe { *folder.0.add(index) };
        if ch == 0 {
            return Ok(result);
        }
        result.push(ch);
    }
    Err(io::Error::other("OS path exceeds native limit"))
}
pub(crate) fn block() -> io::Result<Vec<u16>> {
    let mut system = vec![0u16; 32768];
    // SAFETY: writable output buffer; the OS resolves its own Windows directory.
    let count = unsafe { GetWindowsDirectoryW(system.as_mut_ptr(), system.len() as u32) } as usize;
    if count == 0 || count >= system.len() {
        return Err(io::Error::last_os_error());
    }
    // Windows resolves the AppContainer profile from LOCALAPPDATA during process creation.
    // This path is not an ACL grant, and no caller environment values are inherited.
    let mut block = "LOCALAPPDATA=".encode_utf16().collect::<Vec<_>>();
    block.extend(local_app_data()?);
    block.push(0);
    block.extend("SystemRoot=".encode_utf16());
    block.extend_from_slice(&system[..count]);
    block.extend([0, 0]);
    Ok(block)
}
