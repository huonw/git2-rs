use std::libc::c_uint;
use std::{ptr, vec, cast};
use std::str::raw::from_c_str;
use ffi;
use signature;
use tree::Tree;
use repository::Repository;
use super::{OID, Signature, raise};

pub struct Commit<'self> {
    commit: *mut ffi::git_commit,
    owner: &'self Repository,
}

impl<'self> Commit<'self> {
    /// get the id of the commit
    #[fixed_stack_segment]
    pub fn id<'r>(&self) -> &'r OID
    {
        unsafe {
            // OID pointer returned by git_commit_id is const pointer
            // so it's safe to use as long as self is alive
            cast::transmute(ffi::git_commit_id(self.commit as *ffi::git_commit))
        }
    }

    /// Get the encoding for the message of the commit,
    /// as a string representing a standard encoding name
    /// The encoding may be None, in that case UTF-8 is assumed
    #[fixed_stack_segment]
    pub fn message_encoding(&self) -> Option<~str>
    {
        unsafe {
            let encoding = ffi::git_commit_message_encoding(self.commit as *ffi::git_commit);
            if encoding == ptr::null() {
                None
            } else {
                Some(from_c_str(encoding))
            }
        }
    }

    /// Get the full message of the commit
    #[fixed_stack_segment]
    pub fn message(&self) -> ~str
    {
        unsafe {
            let message = ffi::git_commit_message(self.commit as *ffi::git_commit);
            from_c_str(message)
        }
    }

    /// Get the committer of a commit
    #[fixed_stack_segment]
    pub fn committer(&self) -> Signature
    {
        unsafe {
            let sig = ffi::git_commit_committer(self.commit as *ffi::git_commit);
            signature::from_c_sig(sig)
        }
    }

    /// Get the author of a commit
    #[fixed_stack_segment]
    pub fn author(&self) -> Signature
    {
        unsafe {
            let sig = ffi::git_commit_author(self.commit  as *ffi::git_commit);
            signature::from_c_sig(sig)
        }
    }

    /// Get the tree pointed to by a commit.
    #[fixed_stack_segment]
    pub fn tree<'r>(&'r self) -> ~Tree<'r>
    {
        unsafe {
            let mut tree = ptr::mut_null();
            if ffi::git_commit_tree(&mut tree, self.commit as *ffi::git_commit) == 0 {
                ~Tree { tree: tree, owner: self.owner }
            } else {
                fail!(~"failed to retrieve tree")
            }
        }
    }

    /// Get the parents of the commit.
    #[fixed_stack_segment]
    pub fn parents<'r>(&'r self) -> ~[~Commit<'r>]
    {
        unsafe {
            let len = ffi::git_commit_parentcount(self.commit as *ffi::git_commit) as uint;
            let mut parents:~[~Commit] = vec::with_capacity(len);
            for i in range(0, len) {
                let mut commit_ptr = ptr::mut_null();
                if ffi::git_commit_parent(&mut commit_ptr,
                                          self.commit as *ffi::git_commit, i as c_uint) == 0 {
                    let commit = ~Commit { commit: commit_ptr, owner: self.owner };
                    parents.push(commit);
                } else {
                    raise();
                    return ~[];
                }
            }

            parents
        }
    }

    /// Get the commit object that is the <n>th generation ancestor
    /// of the commit object, following only the first parents.
    ///
    /// Passing `0` as the generation number returns another instance of the
    /// base commit itself.
    #[fixed_stack_segment]
    pub fn nth_gen_ancestor<'r>(&'r self, n: uint) -> Option<~Commit<'r>>
    {
        let mut ancestor = ptr::mut_null();
        unsafe {
            let res = ffi::git_commit_parent(&mut ancestor,
                                             self.commit as *ffi::git_commit, n as c_uint);
            match res {
                0 => Some( ~Commit { commit: ancestor, owner: self.owner } ),
                ffi::GIT_ENOTFOUND => None,
                _ => {
                    raise();
                    None
                },
            }
        }
    }

    /// Get the oid of parents for the commit. This is different from
    /// parents(&self), which will attempt to load the parent commit from the ODB.
    #[fixed_stack_segment]
    pub fn parents_oid(&self) -> ~[~OID]
    {
        unsafe {
            let len = ffi::git_commit_parentcount(self.commit as *ffi::git_commit) as uint;
            let mut parents:~[~OID] = vec::with_capacity(len);
            for i in range(0, len) {
                let mut oid = OID { id: [0, .. 20] };
                let res_ptr = ffi::git_commit_parent_id(self.commit as *ffi::git_commit,
                                                        i as c_uint);
                if res_ptr == ptr::null() {
                    raise();
                    return ~[];
                } else {
                    ptr::copy_memory(&mut oid, res_ptr as *OID, 1);
                    parents.push(~oid);
                }
            }

            parents
        }
    }
}

#[unsafe_destructor]
impl<'self> Drop for Commit<'self> {
    #[fixed_stack_segment]
    fn drop(&self) {
        unsafe {
            ffi::git_commit_free(self.commit);
        }
    }
}
