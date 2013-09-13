use std::ptr;
use super::{raise, GitError, OID, last_error};
use ffi;
use repository::Repository;
use tree::Tree;

pub struct GitIndex<'self> {
    index: *mut ffi::git_index,
    owner: &'self Repository,
}

impl<'self> GitIndex<'self> {
    /// Add or update an index entry from a file on disk
    ///
    /// The file `path` must be relative to the repository's
    /// working folder and must be readable.
    ///
    /// This method will fail in bare index instances.
    ///
    /// This forces the file to be added to the index, not looking
    /// at gitignore rules.  Those rules can be evaluated through
    /// the status APIs before calling this.
    ///
    /// If this file currently is the result of a merge conflict, this
    /// file will no longer be marked as conflicting.  The data about
    /// the conflict will be moved to the "resolve undo" (REUC) section.
    ///
    /// raises git_error on error
    #[fixed_stack_segment]
    pub fn add_bypath(&self, path: &str) {
        unsafe {
            do path.with_c_str |c_path| {
                if ffi::git_index_add_bypath(self.index, c_path) != 0 {
                    raise()
                }
            }
        }
    }

    /// Remove an index entry corresponding to a file on disk
    ///
    /// The file `path` must be relative to the repository's working folder.  It may exist.
    ///
    /// If this file currently is the result of a merge conflict, this
    /// file will no longer be marked as conflicting.  The data about
    /// the conflict will be moved to the "resolve undo" (REUC) section.
    ///
    /// raises git_error on error
    #[fixed_stack_segment]
    pub fn remove_bypath(&self, path: &str) {
        unsafe {
            do path.with_c_str |c_path| {
                if ffi::git_index_remove_bypath(self.index, c_path) != 0 {
                    raise();
                }
            }
        }
    }

    /// Read a tree into the index file with stats
    ///
    /// The current index contents will be replaced by the specified tree.
    /// raises git_error on error
    #[fixed_stack_segment]
    pub fn read_tree(&self, tree: &Tree) {
        unsafe {
            if ffi::git_index_read_tree(self.index,
                                        tree.tree as *ffi::git_tree) != 0 {
                raise()
            }
        }
    }

    /// Write an existing index object from memory back to disk using an atomic file lock.
    ///
    /// raises git_error on error
    #[fixed_stack_segment]
    pub fn write(&self)
    {
        unsafe {
            if ffi::git_index_write(self.index) != 0 {
                raise()
            }
        }
    }

    /// Write the index as a tree
    ///
    /// This method will scan the index and write a representation
    /// of its current state back to disk; it recursively creates
    /// tree objects for each of the subtrees stored in the index,
    /// and returns the root tree. This is the Tree that can be used e.g. to create a commit.
    ///
    /// The index instance cannot be bare, and needs to be associated
    /// to an existing repository.
    ///
    /// The index must not contain any file in conflict.
    #[fixed_stack_segment]
    pub fn write_tree<'r>(&'r self) -> Result<~Tree<'r>, (~str, GitError)> {
        unsafe {
            let mut oid = OID { id: [0, .. 20] };
            let oid_ptr: *mut OID = &mut oid;
            if ffi::git_index_write_tree(oid_ptr as *mut ffi::Struct_git_oid, self.index) == 0 {
                let mut ptr_to_tree = ptr::mut_null();
                if ffi::git_tree_lookup(&mut ptr_to_tree, self.owner.repo,
                                        oid_ptr as *ffi::Struct_git_oid) == 0 {
                    Ok( ~Tree { tree: ptr_to_tree, owner: self.owner } )
                } else {
                    Err( last_error() )
                }
            } else {
                Err( last_error() )
            }
        }
    }

    /// Clear the contents (all the entries) of an index object.
    /// This clears the index object in memory; changes must be manually
    /// written to disk for them to take effect.
    #[fixed_stack_segment]
    pub fn clear(&self) {
        unsafe {
            ffi::git_index_clear(self.index);
        }
    }
}

#[unsafe_destructor]
impl<'self> Drop for GitIndex<'self> {
    #[fixed_stack_segment]
    fn drop(&self) {
        unsafe {
            ffi::git_index_free(self.index);
        }
    }
}
