use std::{
    ffi::OsStr,
    io,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
};
use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
pub(crate) fn wide(value: impl AsRef<OsStr>) -> io::Result<Vec<u16>> {
    let mut data = value.as_ref().encode_wide().collect::<Vec<_>>();
    if data.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid native path",
        ));
    }
    data.push(0);
    Ok(data)
}
pub(crate) fn checked(value: i32) -> io::Result<()> {
    if value == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
pub(crate) fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: callers transfer a fresh successful Win32 owning handle exactly once.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}
pub(crate) fn raw(handle: &OwnedHandle) -> HANDLE {
    handle.as_raw_handle()
}

pub(crate) fn duplicate(handle: &OwnedHandle) -> io::Result<OwnedHandle> {
    use windows_sys::Win32::{
        Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle},
        System::Threading::GetCurrentProcess,
    };
    let mut copy = std::ptr::null_mut();
    // SAFETY: duplicate one live owned handle within this process; the copy is explicitly non-inheritable.
    checked(unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            raw(handle),
            GetCurrentProcess(),
            &mut copy,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    })?;
    owned(copy)
}
