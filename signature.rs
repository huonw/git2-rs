use std::str::raw::from_c_str;
use ffi;
use super::{Signature, Time};

pub fn to_c_sig(_sig: &Signature) -> ffi::git_signature {
    fail!("This is broken due to lifetimes.")
    /*do sig.name.with_c_str |c_name| {
        do sig.email.with_c_str |c_email| {
            ffi::Struct_git_signature {
                name: c_name,
                email: c_email,
                when: ffi::Struct_git_time {
                    time: sig.when.time,
                    offset: sig.when.offset as c_int,
                }
            }
        }
    }*/
}

pub unsafe fn from_c_sig(c_sig: *ffi::git_signature) -> Signature {
    Signature {
        name: from_c_str((*c_sig).name as *i8),
        email: from_c_str((*c_sig).email as *i8),
        when: Time { time: (*c_sig).when.time, offset: (*c_sig).when.offset as int }
    }
}

#[inline]
fn time_cmp(a: &Time, b: &Time) -> i64 {
    let a_utc = a.time + (a.offset as i64) * 60;
    let b_utc = b.time + (b.offset as i64) * 60;
    return a_utc - b_utc;
}

impl Eq for Time {
    fn eq(&self, other: &Time) -> bool {
        self.equals(other)
    }
}

impl Ord for Time {
    fn lt(&self, other: &Time) -> bool {
        self.cmp(other) == Less
    }
}

impl TotalEq for Time {
    fn equals(&self, other: &Time) -> bool {
        self.cmp(other) == Equal
    }
}

impl TotalOrd for Time {
    fn cmp(&self, other: &Time) -> Ordering {
        let res = time_cmp(self, other);
        if res < 0 {
            Less
        } else if res == 0 {
            Equal
        } else {
            Greater
        }
    }
}
