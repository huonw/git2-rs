use super::{GitIndex, Tree, OID};
use ext;

use conditions;

macro_rules! raise {
    ($cond_expr:expr) => ({
        let err = ext::giterr_last();
        let message = str::raw::from_c_str((*err).message);
        let klass = (*err).klass;
        $cond_expr.raise((message, klass))
    })
}

impl GitIndex {
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
    /// raises index_fail on error
    pub fn add_bypath(&mut self, path: &str) {
        unsafe {
            do str::as_c_str(path) |c_path| {
                if ext::git_index_add_bypath(self.index, c_path) != 0 {
                    raise!(conditions::index_fail::cond);
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
    /// raises index_fail on error
    pub fn remove_bypath(&mut self, path: &str) {
        unsafe {
            do str::as_c_str(path) |c_path| {
                if ext::git_index_remove_bypath(self.index, c_path) != 0 {
                    raise!(conditions::index_fail::cond);
                }
            }
        }
    }

    /// Read a tree into the index file with stats
    ///
    /// The current index contents will be replaced by the specified tree.
    /// raises index_fail on error
    pub fn read_tree(&mut self, tree: &Tree) {
        unsafe {
            if ext::git_index_read_tree(self.index, tree.tree) != 0 {
                raise!(conditions::index_fail::cond);
            }
        }
    }

    /// Write an existing index object from memory back to disk using an atomic file lock.
    ///
    /// raises index_fail on error
    pub fn write(&self)
    {
        unsafe {
            if ext::git_index_write(self.index) != 0 {
                raise!(conditions::index_fail::cond)
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
    pub fn write_tree(&self) -> ~Tree {
        unsafe {
            let mut oid = OID { id: [0, .. 20] };
            if ext::git_index_write_tree(&mut oid, self.index) == 0 {
                let mut ptr_to_tree: *ext::git_tree = ptr::null();
                if ext::git_tree_lookup(&mut ptr_to_tree, self.owner.repo, &oid) == 0 {
                    ~Tree { tree: ptr_to_tree, owner: self.owner }
                } else {
                    raise!(conditions::bad_tree::cond)
                }
            } else {
                raise!(conditions::bad_tree::cond)
            }
        }
    }

    /// Clear the contents (all the entries) of an index object.
    /// This clears the index object in memory; changes must be manually
    /// written to disk for them to take effect.
    pub fn clear(&mut self) {
        unsafe {
            ext::git_index_clear(self.index);
        }
    }
}

#[unsafe_destructor]
impl Drop for GitIndex {
    fn finalize(&self) {
        unsafe {
            ext::git_index_free(self.index);
        }
    }
}
