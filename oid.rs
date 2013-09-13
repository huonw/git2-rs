use std::libc::c_char;
use std::{from_str, to_str};
use std::{vec, cast};
use super::{OID, raise};
use ffi;
#[fixed_stack_segment]
fn from_str(s: &str) -> OID {
    unsafe {
        let mut oid = OID { id: [0, .. 20] };
        let oid_ptr: *mut OID = &mut oid;
        do s.with_c_str |c_str| {
            if ffi::git_oid_fromstr(oid_ptr as *mut ffi::git_oid, c_str) != 0 {
                raise()
            }
        }
        return oid;
    }
}

impl from_str::FromStr for OID {
    #[fixed_stack_segment]
    fn from_str(s: &str) -> Option<OID> {
        unsafe {
            let mut oid = OID { id: [0, .. 20] };
            let oid_ptr: *mut OID = &mut oid;
            do s.with_c_str |c_str| {
                if ffi::git_oid_fromstr(oid_ptr as *mut ffi::git_oid, c_str) == 0 {
                    Some(oid)
                } else {
                    None
                }
            }
        }
    }
}

impl to_str::ToStr for OID {
    #[fixed_stack_segment]
    fn to_str(&self) -> ~str {
        let mut v: ~[c_char] = vec::with_capacity(41);
        unsafe {
            let this: *OID = self;
            do v.as_mut_buf |vbuf, _len| {
                ffi::git_oid_fmt(vbuf, this as *ffi::git_oid)
            };
            vec::raw::set_len(&mut v, 40);
            v.push(0);

            return cast::transmute(v);
        }
    }
}

/* from <git2/oid.h> */
#[inline]
fn git_oid_cmp(a: &OID, b: &OID) -> int {
    let mut idx = 0u;
    while idx < 20u {
        if a.id[idx] != b.id[idx] {
            return (a.id[idx] as int) - (b.id[idx] as int)
        }
        idx += 1;
    }
    return 0;
}

impl Eq for OID {
    fn eq(&self, other: &OID) -> bool {
        self.equals(other)
    }
}

impl Ord for OID {
    fn lt(&self, other: &OID) -> bool {
        self.cmp(other) == Less
    }
}
impl TotalEq for OID {
    fn equals(&self, other: &OID) -> bool {
        self.cmp(other) == Equal
    }
}
impl TotalOrd for OID {
    fn cmp(&self, other: &OID) -> Ordering {
        let cmp = git_oid_cmp(self, other);
        if cmp < 0 {
            Less
        } else if cmp == 0 {
            Equal
        } else {
            Greater
        }
    }
}
