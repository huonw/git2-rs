use std::libc::{c_char, c_int};
use std::ptr;
use std::str::raw::from_c_str;
use super::{OID, raise};
use repository::Repository;
use ffi;

pub struct Reference<'self> {
    c_ref: *mut ffi::git_reference,
    owner: &'self Repository,
}

/// Delete the branch reference.
#[fixed_stack_segment]
pub fn branch_delete(reference: &Reference) {
    unsafe {
        if ffi::git_branch_delete(reference.c_ref) != 0 {
            raise();
        }
    }
}

impl<'self> Reference<'self> {
    ///
    /// Return the name of the given local or remote branch.
    ///
    /// The name of the branch matches the definition of the name for branch_lookup.
    /// That is, if the returned name is given to branch_lookup() then the reference is
    /// returned that was given to this function.
    ///
    /// return Some(~str) on success; otherwise None (if the ref is no local or remote branch).
    ///
    #[fixed_stack_segment]
    pub fn branch_name(&self) -> Option<~str> {
        unsafe {
            let mut ptr_to_name: *c_char = ptr::null();
            if ffi::git_branch_name(&mut ptr_to_name, self.c_ref) == 0 {
                Some(from_c_str(ptr_to_name))
            } else {
                None
            }
        }
    }

    /// Determine if the current local branch is pointed at by HEAD.
    #[fixed_stack_segment]
    pub fn is_head(&self) -> bool {
        unsafe {
            match ffi::git_branch_is_head(self.c_ref) {
                1 => true,
                0 => false,
                _ => { raise(); false },
            }
        }
    }

    /// Move/rename an existing local branch reference.
    ///
    /// The new branch name will be checked for validity.
    /// See `git_tag_create()` for rules about valid names.
    #[fixed_stack_segment]
    pub fn branch_move(&self, new_branch_name: &str, force: bool) -> Option<Reference<'self>>
    {
        let mut ptr = ptr::mut_null();
        let flag = force as c_int;
        unsafe {
            do new_branch_name.with_c_str |c_name| {
                let res = ffi::git_branch_move(&mut ptr, self.c_ref, c_name, flag);
                match res {
                    0 => Some( Reference { c_ref: ptr, owner: self.owner } ),
                    ffi::GIT_EINVALIDSPEC => None,
                    _ => { raise(); None },
                }
            }
        }
    }

    /// Return the reference supporting the remote tracking branch,
    /// returns None when the upstream is not found
    #[fixed_stack_segment]
    pub fn upstream(&self) -> Option<Reference<'self>>
    {
        let mut ptr = ptr::mut_null();
        unsafe {
            let res = ffi::git_branch_upstream(&mut ptr, self.c_ref);
            match res {
                0 => Some( Reference { c_ref: ptr, owner: self.owner } ),
                ffi::GIT_ENOTFOUND => None,
                _ => { raise(); None },
            }
        }
    }

    /// Set the upstream configuration for a given local branch
    /// upstream_name: remote-tracking or local branch to set as
    ///     upstream. Pass None to unset.
    #[fixed_stack_segment]
    pub fn set_upstream(&self, upstream_name: Option<&str>)
    {
        let f = |c_name| unsafe {
            if ffi::git_branch_set_upstream(self.c_ref, c_name) == 0 {
                ()
            } else {
                raise()
            }
        };

        match upstream_name {
            None => f(ptr::null()),
            Some(nameref) => nameref.with_c_str(f),
        }
    }

    #[fixed_stack_segment]
    pub fn resolve(&self) -> OID {
        unsafe {
            let mut resolved_ref = ptr::mut_null();
            let mut oid = OID { id: [0, .. 20] };
            if ffi::git_reference_resolve(&mut resolved_ref,
                                          self.c_ref as *ffi::git_reference) == 0 {
                let result_oid = ffi::git_reference_target(resolved_ref as *ffi::git_reference);
                if result_oid == ptr::null() {
                    raise();
                } else {
                    ptr::copy_memory(&mut oid, result_oid as *OID, 1);
                    ffi::git_reference_free(resolved_ref);
                }
            } else {
                raise();
            }
            return oid;
        }
    }
}

#[unsafe_destructor]
impl<'self> Drop for Reference<'self> {
    #[fixed_stack_segment]
    fn drop(&self) {
        unsafe {
            ffi::git_reference_free(self.c_ref);
        }
    }
}
