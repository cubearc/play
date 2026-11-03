use super::handles::wide;
use std::{io, path::Path, ptr};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{
        Authorization::*, DACL_SECURITY_INFORMATION, Isolation::*, PSID,
        SUB_CONTAINERS_AND_OBJECTS_INHERIT,
    },
    Storage::FileSystem::{FILE_GENERIC_EXECUTE, FILE_GENERIC_READ},
};

pub(crate) struct Profile {
    pub sid: PSID,
    name: Vec<u16>,
}
impl Profile {
    pub fn new() -> io::Result<Self> {
        let name = wide(format!("CubeArc.Play.{}", uuid::Uuid::new_v4().simple()))?;
        let mut sid = ptr::null_mut();
        // SAFETY: nul-terminated owned strings and a live output SID pointer; no capabilities.
        let result = unsafe {
            CreateAppContainerProfile(
                name.as_ptr(),
                name.as_ptr(),
                name.as_ptr(),
                ptr::null(),
                0,
                &mut sid,
            )
        };
        if result < 0 {
            return Err(io::Error::from_raw_os_error(result));
        }
        if sid.is_null() {
            return Err(io::Error::other("missing container SID"));
        }
        Ok(Self { sid, name })
    }
    pub fn read_execute(&self, path: &Path) -> io::Result<()> {
        let path = wide(path)?;
        let mut old_acl = ptr::null_mut();
        let mut descriptor = ptr::null_mut();
        // SAFETY: outputs remain valid until the allocated descriptor is freed below.
        let result = unsafe {
            GetNamedSecurityInfoW(
                path.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut old_acl,
                ptr::null_mut(),
                &mut descriptor,
            )
        };
        if result != 0 {
            return Err(io::Error::from_raw_os_error(result as i32));
        }
        let entry = EXPLICIT_ACCESS_W {
            grfAccessPermissions: FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
            grfAccessMode: GRANT_ACCESS,
            grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: ptr::null_mut(),
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_UNKNOWN,
                ptstrName: self.sid.cast(),
            },
        };
        let mut new_acl = ptr::null_mut();
        // SAFETY: the SID and old ACL are live; Win32 allocates the resulting ACL.
        let result = unsafe { SetEntriesInAclW(1, &entry, old_acl, &mut new_acl) };
        let outcome = if result == 0 {
            // SAFETY: the temporary execution image alone receives read/execute access.
            let code = unsafe {
                SetNamedSecurityInfoW(
                    path.as_ptr(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    new_acl,
                    ptr::null(),
                )
            };
            if code == 0 {
                Ok(())
            } else {
                Err(io::Error::from_raw_os_error(code as i32))
            }
        } else {
            Err(io::Error::from_raw_os_error(result as i32))
        };
        // SAFETY: these pointers were allocated by the corresponding Win32 ACL calls.
        unsafe {
            if !new_acl.is_null() {
                LocalFree(new_acl.cast());
            }
            LocalFree(descriptor.cast());
        }
        outcome
    }
}
impl Drop for Profile {
    fn drop(&mut self) {
        // SAFETY: this freshly created private profile and SID remain owned by this object.
        unsafe {
            DeleteAppContainerProfile(self.name.as_ptr());
            windows_sys::Win32::Security::FreeSid(self.sid);
        }
    }
}
