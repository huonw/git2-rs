use std::libc::{c_char, c_int, c_uint, c_void, size_t};
use std::{ptr, cast, vec};
use std::io::Reader;
use std::str::raw::{from_c_str, from_buf_len};
use std::vec::raw::mut_buf_as_slice;
use ffi;
use signature;
use diff;
use tree::Tree;
use reference::Reference;
use git_index::GitIndex;
use commit::Commit;
use blob::Blob;
use super::{GitError, last_error, raise, OID, Status, Signature, DiffList, DiffDelta, WalkMode};

static PATH_BUF_SZ: uint = 1024u;

pub struct Repository {
    repo: *mut ffi::git_repository,
}

/// Open a git repository.
///
/// The 'path' argument must point to either a git repository folder, or an existing work dir.
///
/// The method will automatically detect if 'path' is a normal
/// or bare repository or raise bad_repo if 'path' is neither.
#[fixed_stack_segment]
pub fn open(path: &str) -> Result<Repository, (~str, GitError)>
{
    unsafe {
        let ptr_to_repo = ptr::mut_null();
        let ptr: *mut *mut c_void = &mut (ptr_to_repo as *mut c_void);
        do path.with_c_str |c_path| {
            if ffi::git_repository_open(ptr, c_path) == 0 {
                Ok( Repository { repo: ptr_to_repo } )
            } else {
                Err( last_error() )
            }
        }
    }
}

/// Creates a new Git repository in the given folder.
/// if is_bare is true, a Git repository without a working directory is
/// created at the pointed path. If false, provided path will be
/// considered as the working directory into which the .git directory
/// will be created.
#[fixed_stack_segment]
pub fn init(path: &str, is_bare: bool) -> Result<Repository, (~str, GitError)>
{
    unsafe {
        let ptr_to_repo = ptr::mut_null();
        let ptr: *mut *mut c_void = &mut (ptr_to_repo as *mut c_void);
        do path.with_c_str |c_path| {
            if ffi::git_repository_init(ptr, c_path, is_bare as c_uint) == 0 {
                Ok( Repository { repo: ptr_to_repo } )
            } else {
                Err( last_error() )
            }
        }
    }
}

/// Look for a git repository and copy its path in the given buffer.
/// The lookup start from base_path and walk across parent directories
/// if nothing has been found. The lookup ends when the first repository
/// is found, or when reaching a directory referenced in ceiling_dirs
/// or when the filesystem changes (in case across_fs is true).
///
/// The method will automatically detect if the repository is bare
/// (if there is a repository).
///
/// ceiling_dirs: A GIT_PATH_LIST_SEPARATOR separated list of
/// absolute symbolic link free paths. The lookup will stop when any
/// of this paths is reached. Note that the lookup always performs on
/// start_path no matter start_path appears in ceiling_dirs ceiling_dirs
/// might be empty string
#[fixed_stack_segment]
pub fn discover(start_path: &str, across_fs: bool, ceiling_dirs: &str) -> Option<~str>
{
    unsafe {
        let mut buf = vec::from_elem(PATH_BUF_SZ, 0u8 as c_char);
        do buf.as_mut_buf |c_path, sz| {
            do start_path.with_c_str |c_start_path| {
                do ceiling_dirs.with_c_str |c_ceiling_dirs| {
                    let result = ffi::git_repository_discover(c_path, sz as size_t,
                                            c_start_path, across_fs as c_int, c_ceiling_dirs);
                    if result == 0 {
                        Some( from_c_str(c_path as *c_char) )
                    } else {
                        None
                    }
                }
            }
        }
    }
}

/// Clone a remote repository, and checkout the branch pointed to by the remote
/// this function do not receive options for now
#[fixed_stack_segment]
pub fn clone(url: &str, local_path: &str) -> Result<Repository, (~str, GitError)> {
    unsafe {
        let ptr_to_repo = ptr::mut_null();
        let ptr: *mut *mut c_void = &mut (ptr_to_repo as *mut c_void);
        do url.with_c_str |c_url| {
            do local_path.with_c_str |c_path| {
                if ffi::git_clone(ptr, c_url, c_path, ptr::null()) == 0 {
                    Ok( Repository { repo: ptr_to_repo } )
                } else {
                    Err( last_error() )
                }
            }
        }
    }
}

impl Repository {
    /// Get the path of this repository
    ///
    /// This is the path of the `.git` folder for normal repositories,
    /// or of the repository itself for bare repositories.
    #[fixed_stack_segment]
    pub fn path(&self) -> ~str {
        unsafe {
            let c_path = ffi::git_repository_path(self.repo);
            from_c_str(c_path)
        }
    }

    /// Get the path of the working directory for this repository
    ///
    /// If the repository is bare, this function will always return None.
    #[fixed_stack_segment]
    pub fn workdir(&self) -> Option<~str> {
        unsafe {
            let c_path = ffi::git_repository_workdir(self.repo);
            if ptr::is_null(c_path) {
                None
            } else {
                Some(from_c_str(c_path))
            }
        }
    }

    /// Retrieve and resolve the reference pointed at by HEAD.
    #[fixed_stack_segment]
    pub fn head<'r>(&'r self) -> Option<~Reference<'r>> {
        unsafe {
            let ptr_to_ref = ptr::mut_null();
            let ptr: *mut *mut c_void = &mut (ptr_to_ref as *mut c_void);
            match ffi::git_repository_head(ptr, self.repo) {
                0 => Some( ~Reference { c_ref: ptr_to_ref, owner: self } ),
                ffi::GIT_EORPHANEDHEAD => None,
                ffi::GIT_ENOTFOUND => None,
                _ => {
                    raise();
                    None
                },
            }
        }
    }

    /// Lookup a reference by name in a repository.
    /// The name will be checked for validity.
    #[fixed_stack_segment]
    pub fn lookup<'r>(&'r self, name: &str) -> Option<~Reference<'r>> {
        unsafe {
            let ptr_to_ref = ptr::mut_null();
            let ptr: *mut *mut c_void = &mut (ptr_to_ref as *mut c_void);
            do name.with_c_str |c_name| {
                if(ffi::git_reference_lookup(ptr, self.repo, c_name) == 0) {
                    Some( ~Reference { c_ref: ptr_to_ref, owner: self } )
                } else {
                    None
                }
            }
        }
    }

    /// Lookup a branch by its name in a repository.
    ///
    /// The generated reference must be freed by the user.
    ///
    /// The branch name will be checked for validity.
    /// See `git_tag_create()` for rules about valid names.
    ///
    /// Returns None if the branch name is invalid, or the branch is not found
    ///
    /// remote: True if you want to consider remote branch,
    ///     or false if you want to consider local branch
    #[fixed_stack_segment]
    pub fn lookup_branch<'r>(&'r self, branch_name: &str, remote: bool) -> Option<~Reference<'r>>
    {
        let ptr_to_ref = ptr::mut_null();
        let ptr: *mut *mut c_void = &mut (ptr_to_ref as *mut c_void);
        let branch_type = if remote { ffi::GIT_BRANCH_REMOTE } else { ffi::GIT_BRANCH_LOCAL };
        do branch_name.with_c_str |c_name| {
            unsafe {
                let res = ffi::git_branch_lookup(ptr, self.repo, c_name, branch_type);
                match res {
                    0 => Some( ~Reference { c_ref: ptr_to_ref, owner: self } ),
                    ffi::GIT_ENOTFOUND => None,
                    ffi::GIT_EINVALIDSPEC => None,
                    _ => { raise(); None },
                }
            }
        }
    }

    /// Lookup a commit object from repository
    #[fixed_stack_segment]
    pub fn lookup_commit<'r>(&'r self, id: &OID) -> Option<~Commit<'r>> {
        unsafe {
            let commit = ptr::mut_null();
            let ptr: *mut *mut c_void = &mut (commit as *mut c_void);
            let id_ptr: *OID = id;
            if ffi::git_commit_lookup(ptr, self.repo, id_ptr as *ffi::Struct_git_oid) == 0 {
                Some( ~Commit { commit: commit, owner: self } )
            } else {
                None
            }
        }

    }

    /// Lookup a tree object from repository
    #[fixed_stack_segment]
    pub fn lookup_tree<'r>(&'r self, id: &OID) -> Option<~Tree<'r>> {
        unsafe {
            let tree = ptr::mut_null();
            let ptr: *mut *mut c_void = &mut (tree as *mut c_void);
            let id_ptr: *OID = id;
            if ffi::git_tree_lookup(ptr, self.repo, id_ptr as *ffi::Struct_git_oid) == 0 {
                Some( ~Tree { tree: tree, owner: self } )
            } else {
                None
            }
        }
    }

    /// Updates files in the index and the working tree to match the content of
    /// the commit pointed at by HEAD.
    /// This function does not accept options for now
    ///
    /// returns true when successful, false if HEAD points to an non-existing branch
    /// raise on other errors
    #[fixed_stack_segment]
    pub fn checkout_head(&self) -> bool {
        unsafe {
            match ffi::git_checkout_head(self.repo, ptr::mut_null()) {
                0 => true,
                ffi::GIT_EORPHANEDHEAD => false,
                _ => {
                    raise();
                    false
                }
            }
        }
    }

    /// Get the Index file for this repository.
    ///
    /// If a custom index has not been set, the default
    /// index for the repository will be returned (the one
    /// located in `.git/index`).
    #[fixed_stack_segment]
    pub fn index<'r>(&'r self) -> Result<~GitIndex<'r>, (~str, GitError)> {
        unsafe {
            let ptr_to_ref = ptr::mut_null();
            let ptr: *mut *mut c_void = &mut (ptr_to_ref as *mut c_void);
            if ffi::git_repository_index(ptr, self.repo) == 0 {
                Ok( ~GitIndex { index: ptr_to_ref, owner: self } )
            } else {
                Err( last_error() )
            }
        }
    }

    /// Check if a repository is empty
    #[fixed_stack_segment]
    pub fn is_empty(&self) -> bool {
        unsafe {
            let res = ffi::git_repository_is_empty(self.repo);
            if res < 0 {
                raise();
                false
            } else {
                res != 0
            }
        }
    }

    /// Check if a repository is bare
    #[fixed_stack_segment]
    pub fn is_bare(&self) -> bool {
        unsafe {
            ffi::git_repository_is_bare(self.repo) != 0
        }
    }

    /// Gather file statuses and run a callback for each one.
    /// The callback is passed the path of the file and the status (Status)
    /// If the callback returns false, this function will stop looping
    ///
    /// return values:
    ///   Ok(true): the loop finished successfully
    ///   Ok(false): the callback returned false
    ///   Err(e): found libgit2 errors
    ///
    /// This method is unsafe, as it blocks other tasks while running
    #[fixed_stack_segment]
    pub unsafe fn each_status(&self,
                            op: &fn(path: ~str, status_flags: c_uint) -> bool)
                            -> bool
    {
        let fptr: *mut c_void = cast::transmute(&op);
        let res = ffi::git_status_foreach(self.repo, git_status_cb, fptr);
        if res == 0 {
            true
        } else if res == ffi::GIT_EUSER {
            false
        } else {
            raise();
            false
        }
    }

    /// Safer variant of each_status
    pub fn status(&self) -> ~[(~str, ~Status)] {
        let mut status_list:~[(~str, ~Status)] = ~[];
        unsafe {
            do self.each_status |path, status_flags| {
                let status = ~Status {
                    index_new: status_flags & ffi::GIT_STATUS_INDEX_NEW != 0,
                    index_modified: status_flags & ffi::GIT_STATUS_INDEX_MODIFIED != 0,
                    index_deleted: status_flags & ffi::GIT_STATUS_INDEX_DELETED != 0,
                    index_renamed: status_flags & ffi::GIT_STATUS_INDEX_RENAMED != 0,
                    index_typechange: status_flags & ffi::GIT_STATUS_INDEX_TYPECHANGE != 0,
                    wt_new: status_flags & ffi::GIT_STATUS_WT_NEW != 0,
                    wt_modified: status_flags & ffi::GIT_STATUS_WT_MODIFIED != 0,
                    wt_deleted: status_flags & ffi::GIT_STATUS_WT_DELETED != 0,
                    wt_typechange: status_flags & ffi::GIT_STATUS_WT_TYPECHANGE != 0,
                    ignored: status_flags & ffi::GIT_STATUS_IGNORED != 0,
                };
                status_list.push((path, status));
                true
            };
        }
        status_list
    }


    /// Create a new branch pointing at a target commit
    ///
    /// A new direct reference will be created pointing to
    /// this target commit. If `force` is true and a reference
    /// already exists with the given name, it'll be replaced.
    ///
    /// The returned reference must be freed by the user.
    ///
    /// The branch name will be checked for validity.
    /// See `git_tag_create()` for rules about valid names.
    #[fixed_stack_segment]
    pub fn branch_create<'r>(&'r mut self, branch_name: &str, target: &Commit, force: bool)
        -> Option<~Reference<'r>>
    {
        let ptr_to_ref = ptr::mut_null();
        let ptr: *mut *mut c_void = &mut (ptr_to_ref as *mut c_void);
        let flag = force as c_int;
        unsafe {
            do branch_name.with_c_str |c_name| {
                let res = ffi::git_branch_create(ptr, self.repo, c_name, target.commit as *c_void,
                                                 flag);
                match res {
                    0 => Some( ~Reference { c_ref: ptr_to_ref, owner: self } ),
                    ffi::GIT_EINVALIDSPEC => None,
                    _ => { raise(); None },
                }
            }
        }
    }

    /// Loop over all the branches and issue a callback for each one.
    #[fixed_stack_segment]
    pub fn branch_foreach(&self, local: bool, remote: bool,
        op: &fn(name: &str, is_remote: bool) -> bool) -> bool
    {
        let flocal = if local { ffi::GIT_BRANCH_LOCAL } else { 0 };
        let fremote = if remote { ffi::GIT_BRANCH_REMOTE } else { 0 };
        let flags = flocal & fremote;
        unsafe {
            let payload: *mut c_void = cast::transmute(&op);
            let res = ffi::git_branch_foreach(self.repo, flags, git_branch_foreach_cb, payload);
            match res {
                0 => true,
                ffi::GIT_EUSER => false,
                _ => { raise(); false },
            }
        }
    }

    /// Return the name of the reference supporting the remote tracking branch,
    /// given the name of a local branch reference.
    #[fixed_stack_segment]
    pub fn upstream_name(&self, canonical_branch_name: &str) -> Option<~str>
    {
        let mut buf: [c_char, ..1024] = [0, ..1024];
        do canonical_branch_name.with_c_str |c_name| {
            do buf.as_mut_buf |v, _len| {
                unsafe {
                    let res = ffi::git_branch_upstream_name(v, 1024, self.repo, c_name);
                    if res >= 0 {
                        let ptr = v as *u8;
                        Some( from_buf_len(ptr, res as uint) )
                    } else if res == ffi::GIT_ENOTFOUND {
                        None
                    } else {
                        raise();
                        None
                    }
                }
            }
        }
    }

    /// Return the name of remote that the remote tracking branch belongs to.
    /// returns Err(GIT_ENOTFOUND) when no remote matching remote was found,
    /// returns Err(GIT_EAMBIGUOUS) when the branch maps to several remotes,
    #[fixed_stack_segment]
    pub fn git_branch_remote_name(&self, canonical_branch_name: &str)
        -> Result<~str, (~str, GitError)>
    {
        let mut buf: [c_char, ..1024] = [0, ..1024];
        do canonical_branch_name.with_c_str |c_name| {
            do buf.as_mut_buf |v, _len| {
                unsafe {
                    let res = ffi::git_branch_remote_name(v, 1024, self.repo, c_name);
                    if res >= 0 {
                        let ptr = v as *u8;
                        Ok( from_buf_len(ptr, res as uint) )
                    } else {
                        Err( last_error() )
                    }
                }
            }
        }
    }

    /// Lookup a blob object from a repository.
    #[fixed_stack_segment]
    pub fn blob_lookup<'r>(&'r self, id: &OID) -> Option<~Blob<'r>>
    {
        let ptr_to_blob = ptr::mut_null();
        let ptr: *mut *mut c_void = &mut (ptr_to_blob as *mut c_void);
        let id_ptr: *OID = id;
        unsafe {
            if ffi::git_blob_lookup(ptr, self.repo, id_ptr as *ffi::Struct_git_oid) == 0 {
                Some( ~Blob { blob: ptr_to_blob, owner: self } )
            } else {
                None
            }
        }
    }

    /// Read a file from the working folder of a repository
    /// and write it to the Object Database as a loose blob
    #[fixed_stack_segment]
    pub fn blob_create_fromworkdir<'r>(&'r self, relative_path: &str)
        -> Result<~Blob<'r>, (~str, GitError)>
    {
        let mut oid = OID { id: [0, ..20] };
        let ptr_to_blob = ptr::mut_null();
        let ptr: *mut *mut c_void = &mut (ptr_to_blob as *mut c_void);
        do relative_path.with_c_str |c_path| {
            unsafe {
                let oid_ptr: *mut OID = &mut oid;
                if ffi::git_blob_create_fromworkdir(oid_ptr as *mut ffi::Struct_git_oid,
                                                    self.repo, c_path) == 0 {
                    if ffi::git_blob_lookup(ptr, self.repo, oid_ptr as *ffi::Struct_git_oid) != 0 {
                        fail!(~"blob lookup failure");
                    }
                    Ok( ~Blob { blob: ptr_to_blob, owner: self } )
                } else {
                    Err( last_error() )
                }
            }
        }
    }

    /// Read a file from the filesystem and write its content
    /// to the Object Database as a loose blob
    #[fixed_stack_segment]
    pub fn blob_create_fromdisk<'r>(&'r self, relative_path: &str)
        -> Result<~Blob<'r>, (~str, GitError)>
    {
        let mut oid = OID { id: [0, ..20] };
        let ptr_to_blob = ptr::mut_null();
        let ptr: *mut *mut c_void = &mut (ptr_to_blob as *mut c_void);
        do relative_path.with_c_str |c_path| {
            unsafe {
                let oid_ptr: *mut OID = &mut oid;
                if ffi::git_blob_create_fromdisk(oid_ptr as *mut ffi::Struct_git_oid,
                                                 self.repo, c_path) == 0 {
                    if ffi::git_blob_lookup(ptr, self.repo, oid_ptr as *ffi::Struct_git_oid) != 0 {
                        fail!(~"blob lookup failure");
                    }
                    Ok( ~Blob { blob: ptr_to_blob, owner: self } )
                } else {
                    Err( last_error() )
                }
            }
        }
    }

    /// Write a loose blob to the Object Database from a
    /// provider of chunks of data.
    ///
    /// Provided the `hintpath` parameter is not None, its value
    /// will help to determine what git filters should be applied
    /// to the object before it can be placed to the object database.
    #[fixed_stack_segment]
    pub fn blob_create_fromreader<'r>(&'r self, reader: &Reader, hintpath: Option<&str>)
        -> Result<~Blob<'r>, (~str, GitError)>
    {
        let mut oid = OID { id: [0, ..20] };
        let oid_ptr: *mut OID = &mut oid;
        unsafe {
            let c_path =
            match hintpath {
                None => ptr::null(),
                Some(pathref) => pathref.with_c_str(|ptr| {ptr}),
            };
            let payload: *mut c_void = cast::transmute(&reader);
            if (ffi::git_blob_create_fromchunks(oid_ptr as *mut ffi::Struct_git_oid,
                                                self.repo, c_path, git_blob_chunk_cb,
                    payload) == 0) {
                let ptr_to_blob = ptr::mut_null();
                let ptr: *mut *mut c_void = &mut (ptr_to_blob as *mut c_void);
                if ffi::git_blob_lookup(ptr, self.repo, oid_ptr as *ffi::Struct_git_oid) != 0 {
                    fail!(~"blob lookup failure");
                }
                Ok( ~Blob { blob: ptr_to_blob, owner: self } )
            } else {
                Err( last_error() )
            }
        }
    }

    /// Write an in-memory buffer to the ODB as a blob
    #[fixed_stack_segment]
    pub fn blob_create_frombuffer<'r>(&'r self, buffer: &[u8])
        -> Result<~Blob<'r>, (~str, GitError)>
    {
        let mut oid = OID { id: [0, ..20] };
        let oid_ptr: *mut OID = &mut oid;
        do buffer.as_imm_buf |v, len| {
            unsafe {
                let buf:*c_void = cast::transmute(v);
                if ffi::git_blob_create_frombuffer(oid_ptr as *mut ffi::Struct_git_oid,
                                                   self.repo, buf, len as u64) == 0 {
                    let mut ptr = ptr::mut_null();
                    if ffi::git_blob_lookup(&mut ptr, self.repo,
                                            oid_ptr as *ffi::Struct_git_oid) != 0 {
                        fail!(~"blob lookup failure");
                    }
                    Ok( ~Blob { blob: ptr, owner: self } )
                } else {
                    Err( last_error() )
                }
            }
        }
    }

    /// Create new commit in the repository from a list of Commit pointers
    ///
    /// Returns the created commit. The commit will be written to the Object Database and
    ///  the given reference will be updated to point to it
    ///
    /// id: Pointer in which to store the OID of the newly created commit
    ///
    /// update_ref: If not None, name of the reference that
    ///  will be updated to point to this commit. If the reference
    ///  is not direct, it will be resolved to a direct reference.
    ///  Use "HEAD" to update the HEAD of the current branch and
    ///  make it point to this commit. If the reference doesn't
    ///  exist yet, it will be created.
    ///
    /// author: Signature with author and author time of commit
    ///
    /// committer: Signature with committer and commit time of commit
    ///
    /// message_encoding: The encoding for the message in the
    ///  commit, represented with a standard encoding name.
    ///  E.g. "UTF-8". If None, no encoding header is written and
    ///  UTF-8 is assumed.
    ///
    /// message: Full message for this commit
    ///
    /// tree: An instance of a Tree object that will
    ///  be used as the tree for the commit. This tree object must
    ///  also be owned by `self`
    ///
    /// parents: Vector of Commit objects that will be used as the parents for this commit.
    ///  All the given commits must be owned by `self`.
    #[fixed_stack_segment]
    pub fn commit<'r>(&'r self, update_ref: Option<&str>, author: &Signature,
            committer: &Signature, message_encoding: Option<&str>, message: &str, tree: &Tree,
            parents: &[~Commit<'r>]) -> OID
    {
        unsafe {
            let c_ref =
            match update_ref {
                None => ptr::null(),
                Some(uref) => uref.with_c_str(|ptr| {ptr}),
            };
            let c_author = signature::to_c_sig(author);
            let c_committer = signature::to_c_sig(committer);
            let c_encoding =
            match message_encoding {
                None => ptr::null(),
                Some(enc) => enc.with_c_str(|ptr| {ptr}),
            };
            let c_message = message.with_c_str(|ptr| {ptr});
            let mut oid = OID { id: [0, .. 20] };
            let oid_ptr: *mut OID = &mut oid;
            let mut c_parents = do parents.map |p| { p.commit as *ffi::git_commit};
            do c_parents.as_mut_buf |parent_ptr, len| {
                let res = ffi::git_commit_create(oid_ptr as *mut ffi::Struct_git_oid,
                                                 self.repo, c_ref,
                                                 &c_author, &c_committer,
                                                 c_encoding, c_message,
                                                 tree.tree as *c_void,
                                                 len as c_int,
                                                 parent_ptr);
                if res != 0 {
                    raise()
                }
                oid
            }
        }
    }

    ///
    /// Create a diff list with the difference between two tree objects.
    ///
    /// This is equivalent to `git diff <old-tree> <new-tree>`
    ///
    /// The first tree will be used for the "old_file" side of the delta and the
    /// second tree will be used for the "new_file" side of the delta.  You can
    /// pass None to indicate an empty tree, although it is an error to pass
    /// None for both the `old_tree` and `new_tree`.
    ///
    /// @param diff Output pointer to a git_diff_list pointer to be allocated.
    /// @param repo The repository containing the trees.
    /// @param old_tree A git_tree object to diff from, or NULL for empty tree.
    /// @param new_tree A git_tree object to diff to, or NULL for empty tree.
    /// @param opts Structure with options to influence diff or NULL for defaults.
    ///
    #[fixed_stack_segment]
    pub fn diff_tree_to_tree<'r>(&'r self, old_tree: Option<~Tree>, new_tree: Option<~Tree>,
            opts: &diff::DiffOption, notify_cb: &fn(DiffList, DiffDelta, ~str) -> WalkMode)
        -> Result<~DiffList, (~str, GitError)>
    {
        unsafe {
            let old_t = match old_tree {
                None => ptr::mut_null(),
                Some(t) => t.tree,
            };

            let new_t = match new_tree {
                None => ptr::mut_null(),
                Some(t) => t.tree,
            };

            let flags = do opts.flags.iter().fold(0u32) |flags, &f| {
                flags | (f as u32)
            };

            // we need to allocate these separately, to keep the
            // pointers around long enough.
            let c_strs = opts.pathspec.map(|path| path.to_c_str());

            let mut pathspec = do c_strs.map |path| {
                path.with_ref(|ptr| ptr as *mut i8)
            };

            let c_pathspec = ffi::Struct_git_strarray {
                strings: vec::raw::to_mut_ptr(pathspec),
                count: pathspec.len() as u64,
            };

            let c_opts = ffi::git_diff_options {
                version: 1,     // GIT_DIFF_OPTIONS_VERSION
                flags: flags,
                context_lines: opts.context_lines,
                interhunk_lines: opts.interhunk_lines,
                old_prefix: do opts.old_prefix.with_c_str |c_pref| { c_pref },
                new_prefix: do opts.new_prefix.with_c_str |c_pref| { c_pref },
                pathspec: c_pathspec,
                max_size: opts.max_size,
                notify_cb: git_diff_notify_cb,
                notify_payload: cast::transmute(&notify_cb),
                ignore_submodules: ffi::GIT_SUBMODULE_IGNORE_NONE,
            };

            let mut diff_list = ptr::mut_null();

            if ffi::git_diff_tree_to_tree(&mut diff_list,
                                          self.repo, old_t, new_t, &c_opts) == 0 {
                Ok( ~DiffList { difflist: diff_list } )
            } else {
                Err( last_error() )
            }
        }
    }
}

extern "C" fn git_status_cb(path: *c_char, status_flags: c_uint, payload: *mut c_void) -> c_int
{
    unsafe {
        let op_ptr = payload as *&fn(~str, c_uint) -> bool;
        let path_str = from_c_str(path);
        if (*op_ptr)(path_str, status_flags) {
            0
        } else {
            1
        }
    }
}

extern fn git_blob_chunk_cb(content: *mut i8, max_length: size_t, payload: *mut c_void) -> c_int
{
    let len = max_length as uint;
    unsafe {
        let reader = *(payload as *&Reader);
        do mut_buf_as_slice(content as *mut u8, len) |v| {
            if reader.eof() {
                0
            } else {
                reader.read(v, len) as c_int
            }
        }
    }
}

extern fn git_branch_foreach_cb(branch_name: *c_char, branch_type: ffi::git_branch_t,
    payload: *mut c_void) -> c_int
{
    unsafe {
        let op_ptr = payload as *&fn(name: &str, is_remote: bool) -> bool;
        let branch_str = from_c_str(branch_name);
        let is_remote = (branch_type == ffi::GIT_BRANCH_REMOTE);
        if (*op_ptr)(branch_str, is_remote) {
            0
        } else {
            1
        }
    }
}

extern fn git_diff_notify_cb(diff_so_far: *ffi::git_diff_list,
                             _delta_to_add: *ffi::git_diff_delta,
                             matched_pathspec: *c_char, payload: *mut c_void) -> c_int
{
    unsafe {
        let _op = payload as *&fn(DiffList, DiffDelta, ~str) -> bool;
        let _difflist = DiffList { difflist: diff_so_far as *mut ffi::git_diff_list };
        let _spec_str = from_c_str(matched_pathspec);
        //(*op)(difflist, cast::transmute(*delta_to_add), spec_str) as c_int
        fail!("I dunno what this is supposed to be?")
    }
}

impl Drop for Repository {
    #[fixed_stack_segment]
    fn drop(&self) {
        unsafe {
            ffi::git_repository_free(self.repo);
        }
    }
}
