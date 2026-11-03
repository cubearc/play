use super::{
    Limits,
    handles::{checked, owned, raw},
};
use std::{
    io,
    mem::{size_of, zeroed},
    ptr,
};
use windows_sys::Win32::{
    Security::*,
    System::{JobObjects::*, Threading::*},
};
pub fn current(limits: Limits) -> io::Result<()> {
    limits.validate()?;
    let mut token = ptr::null_mut();
    // SAFETY: the current process pseudo-handle is valid and Win32 writes an owning token handle.
    checked(unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) })?;
    let token = owned(token)?;
    let mut flag = 0u32;
    let mut length = 0;
    // SAFETY: exact documented DWORD output type for TokenIsAppContainer.
    checked(unsafe {
        GetTokenInformation(
            raw(&token),
            TokenIsAppContainer,
            (&mut flag as *mut u32).cast(),
            size_of::<u32>() as u32,
            &mut length,
        )
    })?;
    if flag != 1 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "AppContainer is required",
        ));
    }
    // SAFETY: first query obtains the needed TOKEN_GROUPS buffer size without reading data.
    unsafe {
        GetTokenInformation(
            raw(&token),
            TokenCapabilities,
            ptr::null_mut(),
            0,
            &mut length,
        );
    }
    if !(size_of::<u32>() as u32..=65536).contains(&length) {
        return Err(io::Error::other("invalid capability buffer"));
    }
    let mut groups = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
    // SAFETY: aligned buffer of the reported size; only GroupCount is read afterwards.
    checked(unsafe {
        GetTokenInformation(
            raw(&token),
            TokenCapabilities,
            groups.as_mut_ptr().cast(),
            length,
            &mut length,
        )
    })?;
    // SAFETY: TOKEN_GROUPS starts with a DWORD GroupCount; the buffer holds at least that field.
    if unsafe { *(groups.as_ptr().cast::<u32>()) } != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "capabilities are forbidden",
        ));
    }
    // SAFETY: the documented job structure is initialized before querying the caller's job.
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
    // SAFETY: NULL selects the job containing this process; class and size match the structure.
    checked(unsafe {
        QueryInformationJobObject(
            ptr::null_mut(),
            JobObjectExtendedLimitInformation,
            (&mut info as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ptr::null_mut(),
        )
    })?;
    let required = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
        | JOB_OBJECT_LIMIT_PROCESS_MEMORY
        | JOB_OBJECT_LIMIT_PROCESS_TIME;
    if info.BasicLimitInformation.LimitFlags & required != required
        || info.BasicLimitInformation.ActiveProcessLimit != 1
        || info.ProcessMemoryLimit == 0
        || info.ProcessMemoryLimit > limits.private_memory_bytes
        || info.BasicLimitInformation.PerProcessUserTimeLimit <= 0
        || info.BasicLimitInformation.PerProcessUserTimeLimit
            > (limits.cpu_seconds * 10_000_000) as i64
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "bounded job is required",
        ));
    }
    Ok(())
}
