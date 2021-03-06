use std::libc::c_char;
use std::{from_str, to_str};
use std::{vec, cast};
use super::{OID, raise};
use ext;

fn from_str(s: &str) -> OID {
    unsafe {
        let mut oid = OID { id: [0, .. 20] };
        do s.as_c_str |c_str| {
            if ext::git_oid_fromstr(&mut oid, c_str) != 0 {
                raise()
            }
        }
        return oid;
    }
}

impl from_str::FromStr for OID {
    fn from_str(s: &str) -> Option<OID> {
        unsafe {
            let mut oid = OID { id: [0, .. 20] };
            do s.as_c_str |c_str| {
                if ext::git_oid_fromstr(&mut oid, c_str) == 0 {
                    Some(oid)
                } else {
                    None
                }
            }
        }
    }
}

impl to_str::ToStr for OID {
    fn to_str(&self) -> ~str {
        let mut v: ~[c_char] = vec::with_capacity(41);
        unsafe {
            do vec::as_mut_buf(v) |vbuf, _len| {
                ext::git_oid_fmt(vbuf, self)
            };
            vec::raw::set_len(&mut v, 40);
            v.push(0);

            return cast::transmute(v);
        }
    }
}

/* from <git2/oid.h> */
#[inline]
priv fn git_oid_cmp(a: &OID, b: &OID) -> int {
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
        git_oid_cmp(self, other) == 0
    }

    fn ne(&self, other: &OID) -> bool {
        git_oid_cmp(self, other) != 0
    }
}

impl Ord for OID {
    fn lt(&self, other: &OID) -> bool {
        git_oid_cmp(self, other) < 0
    }

    fn le(&self, other: &OID) -> bool {
        git_oid_cmp(self, other) <= 0
    }

    fn gt(&self, other: &OID) -> bool {
        git_oid_cmp(self, other) > 0
    }

    fn ge(&self, other: &OID) -> bool {
        git_oid_cmp(self, other) >= 0
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
