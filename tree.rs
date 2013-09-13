use std::libc::{c_void, c_char, c_int};
use std::{ptr, cast};
use std::str::raw::from_c_str;
use ffi;
use super::{WalkMode, OID, OType, raise, last_error, GitError, FileMode};
use repository::Repository;

pub struct Tree<'self> {
    tree: *mut ffi::git_tree,
    owner: &'self Repository,
}

pub struct TreeEntry {
    tree_entry: *mut ffi::git_tree_entry,
    owned: bool,
}

pub struct TreeBuilder {
    bld: *mut ffi::git_treebuilder,
}

impl TreeBuilder {
    /// Create a new tree builder.
    /// The tree builder can be used to create or modify trees in memory and
    /// write them as tree objects to the database.
    /// The tree builder will start with no entries and will have to be filled manually.
    #[fixed_stack_segment]
    pub fn new() -> TreeBuilder
    {
        let mut bld =  ptr::mut_null();
        unsafe {
            if ffi::git_treebuilder_create(&mut bld, ptr::null()) == 0 {
                TreeBuilder { bld: bld }
            } else {
                fail!(~"failed to create treebuilder")
            }
        }
    }

    /// Create a new tree builder.
    /// The tree builder will be initialized with the entries of the given tree.
    #[fixed_stack_segment]
    pub fn from_tree(tree: &Tree) -> TreeBuilder
    {
        let mut bld = ptr::mut_null();
        unsafe {
            if ffi::git_treebuilder_create(&mut bld, tree.tree as *ffi::git_tree) == 0 {
                TreeBuilder { bld: bld }
            } else {
                fail!(~"failed to create treebuilder")
            }
        }
    }
}


impl<'self> Tree<'self> {
    /// Get the id of a tree.
    #[fixed_stack_segment]
    pub fn id<'r>(& self) -> &'r OID
    {
        unsafe {
            cast::transmute(ffi::git_tree_id(self.tree as *ffi::git_tree))
        }
    }

    /// Lookup a tree entry by its filename
    #[fixed_stack_segment]
    pub fn entry_byname(&self, filename: &str) -> Option<~TreeEntry>
    {
        do filename.with_c_str |c_filename| {
            unsafe {
                let entry_ptr = ffi::git_tree_entry_byname(self.tree as *ffi::git_tree,
                                                           c_filename);
                if entry_ptr == ptr::null() {
                    None
                } else {
                    Some(~TreeEntry{
                            tree_entry: entry_ptr as *mut ffi::git_tree_entry,
                            owned: false
                        })
                }
            }
        }
    }

    /// Lookup a tree entry by SHA value.
    /// Warning: this must examine every entry in the tree, so it is not fast.
    #[fixed_stack_segment]
    pub fn entry_byoid(&self, oid: &OID) -> Option<~TreeEntry>
    {
        let oid_ptr: *OID = oid;
        unsafe {
            let entry_ptr = ffi::git_tree_entry_byoid(self.tree as *ffi::git_tree,
                                                      oid_ptr as *ffi::git_oid);
            if entry_ptr == ptr::null() {
                None
            } else {
                Some( ~TreeEntry{
                        tree_entry: entry_ptr as *mut ffi::git_tree_entry,
                        owned: false
                    })
            }
        }
    }

    /// Retrieve a tree entry contained in a tree or in any of its subtrees,
    /// given its relative path.
    #[fixed_stack_segment]
    pub fn entry_bypath(&self, path: &str) -> Option<~TreeEntry>
    {
        do path.with_c_str |c_path| {
            unsafe {
                let mut entry_ptr = ptr::mut_null();
                if ffi::git_tree_entry_bypath(&mut entry_ptr, self.tree as *ffi::git_tree,
                                              c_path) == 0 {
                    Some( ~TreeEntry{tree_entry: entry_ptr, owned: true} )
                } else {
                    None
                }
            }
        }
    }

    /// Traverse the entries in a tree and its subtrees in pre order.
    ///
    /// Children subtrees will be automatically loaded as required, and the `callback` will be
    /// called once per entry with the current (relative) root for the entry and
    /// the entry data itself.
    ///
    /// If the callback returns WalkSkip, the passed entry will be skipped on the traversal.
    /// WalkPass continues the walk, and WalkStop stops the walk.
    ///
    /// The function returns false if the loop is stopped by StopWalk
    #[fixed_stack_segment]
    pub fn walk_preorder(&self, callback: &fn(&str, &TreeEntry) -> WalkMode) -> bool
    {
        unsafe {
            let fptr: *mut c_void = cast::transmute(&callback);
            let result = ffi::git_tree_walk(self.tree as *ffi::git_tree,
                                            ffi::GIT_TREEWALK_PRE, pre_walk_cb, fptr);
            if result == 0 {
                true
            } else if result == ffi::GIT_EUSER {
                false
            } else {
                raise();
                false
            }
        }
    }

    /// Traverse the entries in a tree and its subtrees in post order.
    ///
    /// Children subtrees will be automatically loaded as required, and the `callback` will be
    /// called once per entry with the current (relative) root for the entry and
    /// the entry data itself.
    ///
    /// If the callback returns false, the loop stops
    ///
    /// The function returns false if the loop is stopped by callback
    #[fixed_stack_segment]
    pub fn walk_postorder(&self, callback: &fn(&str, &TreeEntry) -> bool) -> bool
    {
        unsafe {
            let fptr: *mut c_void = cast::transmute(&callback);
            let result = ffi::git_tree_walk(self.tree as *ffi::git_tree,
                                            ffi::GIT_TREEWALK_POST, post_walk_cb, fptr);
            if result == 0 {
                true
            } else if result == ffi::GIT_EUSER {
                false
            } else {
                raise();
                false
            }
        }
    }
}

extern fn pre_walk_cb(root: *c_char, entry: *ffi::git_tree_entry, payload: *mut c_void) -> c_int
{
    unsafe {
        let op_ptr = payload as *&fn(&str, &TreeEntry) -> WalkMode;
        let root_str = from_c_str(root);
        let entry = TreeEntry { tree_entry: entry as *mut ffi::git_tree_entry, owned: false };
        (*op_ptr)(root_str, &entry) as c_int
    }
}

extern fn post_walk_cb(root: *c_char, entry: *ffi::git_tree_entry, payload: *mut c_void) -> c_int
{
    unsafe {
        let op_ptr = payload as *&fn(&str, &TreeEntry) -> bool;
        let root_str = from_c_str(root);
        let entry = TreeEntry { tree_entry: entry as *mut ffi::git_tree_entry, owned: false };
        if (*op_ptr)(root_str, &entry) {
            // continue
            0
        } else {
            // negative value stops the walk
            -1
        }
    }
}

/*impl<'self> BaseIter<TreeEntry> for Tree<'self> {
    /// traverse Tree with internal storage order
    fn each(&self, blk: &fn(v: &TreeEntry) -> bool) -> bool {
        unsafe {
            let size = ffi::git_tree_entrycount(self.tree);
            let mut idx:size_t = 0;
            while idx < size {
                let entry_ptr = ffi::git_tree_entry_byindex(self.tree, idx);
                if entry_ptr == ptr::null() {
                    fail!(~"bad entry pointer")
                }
                let entry = TreeEntry { tree_entry: entry_ptr, owned: false };
                if !blk(&entry) {
                    return false;
                }
                idx += 1;
            }
            return true;
        }
    }

    fn size_hint(&self) -> Option<uint> {
        unsafe {
            Some(ffi::git_tree_entrycount(self.tree) as uint)
        }
    }
}*/

#[unsafe_destructor]
impl<'self> Drop for Tree<'self> {
    #[fixed_stack_segment]
    fn drop(&self) {
        unsafe {
            ffi::git_tree_free(self.tree);
        }
    }
}

impl TreeEntry {
    /// Get the filename of a tree entry
    #[fixed_stack_segment]
    pub fn name(&self) -> ~str
    {
        unsafe {
            from_c_str(ffi::git_tree_entry_name(self.tree_entry as *ffi::git_tree_entry))
        }
    }

    /// Get the id of the object pointed by the entry
    #[fixed_stack_segment]
    pub fn id<'r>(&self) -> &'r OID
    {
        unsafe {
            cast::transmute(ffi::git_tree_entry_id(self.tree_entry as *ffi::git_tree_entry))
        }
    }

    #[fixed_stack_segment]
    pub fn otype(&self) -> OType
    {
        unsafe {
            cast::transmute(ffi::git_tree_entry_type(self.tree_entry as *ffi::git_tree_entry) as u64)
        }
    }

    #[fixed_stack_segment]
    pub fn filemode(&self) -> FileMode
    {
        unsafe {
            cast::transmute(ffi::git_tree_entry_filemode(self.tree_entry as *ffi::git_tree_entry) as u64)
        }
    }
}

#[unsafe_destructor]
impl Drop for TreeEntry {
    #[fixed_stack_segment]
    fn drop(&self) {
        unsafe {
            if self.owned {
                ffi::git_tree_entry_free(self.tree_entry);
            }
        }
    }
}

impl Clone for TreeEntry {
    #[fixed_stack_segment]
    fn clone(&self) -> TreeEntry {
        unsafe {
            TreeEntry {
                tree_entry: ffi::git_tree_entry_dup(self.tree_entry as *ffi::git_tree_entry),
                owned: self.owned,
            }
        }
    }
}

#[inline]
#[fixed_stack_segment]
fn tree_entry_cmp(a: &TreeEntry, b: &TreeEntry) -> c_int
{
    unsafe {
        ffi::git_tree_entry_cmp(a.tree_entry as *ffi::git_tree_entry,
                                b.tree_entry as *ffi::git_tree_entry)
    }
}

impl Eq for TreeEntry {
    fn eq(&self, other: &TreeEntry) -> bool {
        self.equals(other)
    }
}

impl Ord for TreeEntry {
    fn lt(&self, other: &TreeEntry) -> bool {
        self.cmp(other) == Less
    }
}

impl TotalEq for TreeEntry {
    fn equals(&self, other: &TreeEntry) -> bool {
        self.cmp(other) == Equal
    }
}

impl TotalOrd for TreeEntry {
    fn cmp(&self, other: &TreeEntry) -> Ordering {
        let comp = tree_entry_cmp(self, other);
        if comp < 0 {
            Less
        } else if comp == 0 {
            Equal
        } else {
            Greater
        }
    }
}

impl TreeBuilder {
    /// Clear all the entires in the builder
    #[fixed_stack_segment]
    pub fn clear(&self)
    {
        unsafe {
            ffi::git_treebuilder_clear(self.bld);
        }
    }

    /// Get an entry from the builder from its filename
    #[fixed_stack_segment]
    pub fn get(&self, filename: &str) -> ~TreeEntry
    {
        do filename.with_c_str |c_filename| {
            unsafe {
                let entry_ptr = ffi::git_treebuilder_get(self.bld, c_filename);
                ~TreeEntry { tree_entry: entry_ptr as *mut ffi::git_tree_entry, owned: false }
            }
        }
    }

    /// Add or update an entry to the builder
    ///
    /// Insert a new entry for `filename` in the builder with the
    /// given attributes.
    ///
    /// If an entry named `filename` already exists, its attributes
    /// will be updated with the given ones.
    ///
    /// No attempt is being made to ensure that the provided oid points
    /// to an existing git object in the object database, nor that the
    /// attributes make sense regarding the type of the pointed at object.
    ///
    /// filename: Filename of the entry
    /// id: SHA1 OID of the entry
    /// filemode: Folder attributes of the entry. This parameter must not be GIT_FILEMODE_NEW
    #[fixed_stack_segment]
    pub fn insert(&self, filename: &str, id: &OID, filemode: FileMode) ->
        Result<~TreeEntry, (~str, GitError)>
    {
        let id_ptr: *OID = id;
        do filename.with_c_str |c_filename| {
            unsafe {
                let mut entry_ptr = ptr::null();
                if(ffi::git_treebuilder_insert(&mut entry_ptr, self.bld, c_filename,
                                               id_ptr as *ffi::Struct_git_oid,
                                               filemode as u32) == 0) {
                    Ok( ~TreeEntry {
                            tree_entry: entry_ptr as *mut ffi::git_tree_entry,
                            owned: false
                        } )
                } else {
                    Err( last_error() )
                }
            }
        }
    }

    /// Remove an entry from the builder by its filename
    /// return true if successful, false if the entry does not exist
    #[fixed_stack_segment]
    pub fn remove(&self, filename: &str) -> bool
    {
        do filename.with_c_str |c_filename| {
            unsafe {
                ffi::git_treebuilder_remove(self.bld, c_filename) == 0
            }
        }
    }

    /// Filter the entries in the tree
    ///
    /// The `filter` closure will be called for each entry in the tree with a
    /// ref to the entry;
    /// if the closure returns false, the entry will be filtered (removed from the builder).
    #[fixed_stack_segment]
    pub fn filter(&self, filter: &fn(&TreeEntry) -> bool)
    {
        unsafe {
            ffi::git_treebuilder_filter(self.bld, filter_cb, cast::transmute(&filter));
        }
    }

    /// Write the contents of the tree builder as a tree object
    ///
    /// The tree builder will be written to the given `repo`, and its
    /// identifying SHA1 hash will be returned
    ///
    /// repo: Repository in which to store the object
    #[fixed_stack_segment]
    pub fn write(&self, repo: &Repository) -> OID
    {
        let mut oid = OID { id: [0, ..20] };
        let oid_ptr: *mut OID = &mut oid;
        unsafe {
            if ffi::git_treebuilder_write(oid_ptr as *mut ffi::Struct_git_oid,
                                          repo.repo, self.bld) != 0 {
                raise()
            }
        }
        return oid;
    }

    /// Get the number of entries listed in a treebuilder
    #[fixed_stack_segment]
    pub fn entrycount(&self) -> uint
    {
        unsafe {
            ffi::git_treebuilder_entrycount(self.bld) as uint
        }
    }
}

extern fn filter_cb(entry: *ffi::git_tree_entry, payload: *mut c_void) -> c_int
{
    unsafe {
        let op_ptr = payload as *&fn(&TreeEntry) -> bool;
        let entry = TreeEntry { tree_entry: entry as *mut ffi::git_tree_entry, owned: false };
        if (*op_ptr)(&entry) {
            0
        } else {
            1
        }
    }
}

#[unsafe_destructor]
impl Drop for TreeBuilder {
    #[fixed_stack_segment]
    fn drop(&self) {
        unsafe {
            ffi::git_treebuilder_free(self.bld);
        }
    }
}
