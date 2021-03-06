use std::libc::{c_char, c_int, c_uint, c_void, size_t};
use std::{ptr, cast};
use std::io::Reader;
use std::str::raw::{from_c_str, from_c_str_len};
use std::vec::raw::mut_buf_as_slice;
use std::vec::{as_mut_buf, as_imm_buf, as_const_buf};
use ext;
use signature;
use super::*;

static PATH_BUF_SZ: uint = 1024u;

/// Open a git repository.
///
/// The 'path' argument must point to either a git repository folder, or an existing work dir.
///
/// The method will automatically detect if 'path' is a normal
/// or bare repository or raise bad_repo if 'path' is neither.
pub fn open(path: &str) -> Result<Repository, (~str, GitError)>
{
    unsafe {
        let mut ptr_to_repo: *ext::git_repository = ptr::null();
        do path.as_c_str |c_path| {
            if ext::git_repository_open(&mut ptr_to_repo, c_path) == 0 {
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
pub fn init(path: &str, is_bare: bool) -> Result<Repository, (~str, GitError)>
{
    unsafe {
        let mut ptr_to_repo: *ext::git_repository = ptr::null();
        do path.as_c_str |c_path| {
            if ext::git_repository_init(&mut ptr_to_repo, c_path, is_bare as c_uint) == 0 {
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
pub fn discover(start_path: &str, across_fs: bool, ceiling_dirs: &str) -> Option<~str>
{
    unsafe {
        let mut buf = std::vec::from_elem(PATH_BUF_SZ, 0u8 as c_char);
        do as_mut_buf(buf) |c_path, sz| {
            do start_path.as_c_str |c_start_path| {
                do ceiling_dirs.as_c_str |c_ceiling_dirs| {
                    let result = ext::git_repository_discover(c_path, sz as size_t,
                                            c_start_path, across_fs as c_int, c_ceiling_dirs);
                    if result == 0 {
                        Some( std::str::raw::from_buf(c_path as *u8) )
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
pub fn clone(url: &str, local_path: &str) -> Result<Repository, (~str, GitError)> {
    unsafe {
        let mut ptr_to_repo: *ext::git_repository = ptr::null();
        do url.as_c_str |c_url| {
            do local_path.as_c_str |c_path| {
                if ext::git_clone(&mut ptr_to_repo, c_url, c_path, ptr::null()) == 0 {
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
    pub fn path(&self) -> ~str {
        unsafe {
            let c_path = ext::git_repository_path(self.repo);
            from_c_str(c_path)
        }
    }

    /// Get the path of the working directory for this repository
    ///
    /// If the repository is bare, this function will always return None.
    pub fn workdir(&self) -> Option<~str> {
        unsafe {
            let c_path = ext::git_repository_workdir(self.repo);
            if ptr::is_null(c_path) {
                None
            } else {
                Some(from_c_str(c_path))
            }
        }
    }

    /// Retrieve and resolve the reference pointed at by HEAD.
    pub fn head<'r>(&'r self) -> Option<~Reference<'r>> {
        unsafe {
            let mut ptr_to_ref: *ext::git_reference = ptr::null();

            match ext::git_repository_head(&mut ptr_to_ref, self.repo) {
                0 => Some( ~Reference { c_ref: ptr_to_ref, owner: self } ),
                ext::GIT_EORPHANEDHEAD => None,
                ext::GIT_ENOTFOUND => None,
                _ => {
                    raise();
                    None
                },
            }
        }
    }

    /// Lookup a reference by name in a repository.
    /// The name will be checked for validity.
    pub fn lookup<'r>(&'r self, name: &str) -> Option<~Reference<'r>> {
        unsafe {
            let mut ptr_to_ref: *ext::git_reference = ptr::null();

            do name.as_c_str |c_name| {
                if(ext::git_reference_lookup(&mut ptr_to_ref, self.repo, c_name) == 0) {
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
    pub fn lookup_branch<'r>(&'r self, branch_name: &str, remote: bool) -> Option<~Reference<'r>>
    {
        let mut ptr: *ext::git_reference = ptr::null();
        let branch_type = if remote { ext::GIT_BRANCH_REMOTE } else { ext::GIT_BRANCH_LOCAL };
        do branch_name.as_c_str |c_name| {
            unsafe {
                let res = ext::git_branch_lookup(&mut ptr, self.repo, c_name, branch_type);
                match res {
                    0 => Some( ~Reference { c_ref: ptr, owner: self } ),
                    ext::GIT_ENOTFOUND => None,
                    ext::GIT_EINVALIDSPEC => None,
                    _ => { raise(); None },
                }
            }
        }
    }

    /// Lookup a commit object from repository
    pub fn lookup_commit<'r>(&'r self, id: &OID) -> Option<~Commit<'r>> {
        unsafe {
            let mut commit: *ext::git_commit = ptr::null();
            if ext::git_commit_lookup(&mut commit, self.repo, id) == 0 {
                Some( ~Commit { commit: commit, owner: self } )
            } else {
                None
            }
        }
    }

    /// Lookup a tree object from repository
    pub fn lookup_tree<'r>(&'r self, id: &OID) -> Option<~Tree<'r>> {
        unsafe {
            let mut tree: *ext::git_tree = ptr::null();
            if ext::git_tree_lookup(&mut tree, self.repo, id) == 0 {
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
    pub fn checkout_head(&self) -> bool {
        unsafe {
            match ext::git_checkout_head(self.repo, ptr::null()) {
                0 => true,
                ext::GIT_EORPHANEDHEAD => false,
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
    pub fn index<'r>(&'r self) -> Result<~GitIndex<'r>, (~str, GitError)> {
        unsafe {
            let mut ptr_to_ref: *ext::git_index = ptr::null();

            if ext::git_repository_index(&mut ptr_to_ref, self.repo) == 0 {
                Ok( ~GitIndex { index: ptr_to_ref, owner: self } )
            } else {
                Err( last_error() )
            }
        }
    }

    /// Check if a repository is empty
    pub fn is_empty(&self) -> bool {
        unsafe {
            let res = ext::git_repository_is_empty(self.repo);
            if res < 0 {
                raise();
                false
            } else {
                res as bool
            }
        }
    }

    /// Check if a repository is bare
    pub fn is_bare(&self) -> bool {
        unsafe {
            ext::git_repository_is_bare(self.repo) as bool
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
    pub unsafe fn each_status(&self,
                            op: &fn(path: ~str, status_flags: c_uint) -> bool)
                            -> bool
    {
        let fptr: *c_void = cast::transmute(&op);
        let res = ext::git_status_foreach(self.repo, git_status_cb, fptr);
        if res == 0 {
            true
        } else if res == ext::GIT_EUSER {
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
            for self.each_status |path, status_flags| {
                let status = ~Status {
                    index_new: status_flags & ext::GIT_STATUS_INDEX_NEW != 0,
                    index_modified: status_flags & ext::GIT_STATUS_INDEX_MODIFIED != 0,
                    index_deleted: status_flags & ext::GIT_STATUS_INDEX_DELETED != 0,
                    index_renamed: status_flags & ext::GIT_STATUS_INDEX_RENAMED != 0,
                    index_typechange: status_flags & ext::GIT_STATUS_INDEX_TYPECHANGE != 0,
                    wt_new: status_flags & ext::GIT_STATUS_WT_NEW != 0,
                    wt_modified: status_flags & ext::GIT_STATUS_WT_MODIFIED != 0,
                    wt_deleted: status_flags & ext::GIT_STATUS_WT_DELETED != 0,
                    wt_typechange: status_flags & ext::GIT_STATUS_WT_TYPECHANGE != 0,
                    ignored: status_flags & ext::GIT_STATUS_IGNORED != 0,
                };
                status_list.push((path, status));
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
    pub fn branch_create<'r>(&'r mut self, branch_name: &str, target: &Commit, force: bool)
        -> Option<~Reference<'r>>
    {
        let mut ptr: *ext::git_reference = ptr::null();
        let flag = force as c_int;
        unsafe {
            do branch_name.as_c_str |c_name| {
                let res = ext::git_branch_create(&mut ptr, self.repo, c_name, target.commit, flag);
                match res {
                    0 => Some( ~Reference { c_ref: ptr, owner: self } ),
                    ext::GIT_EINVALIDSPEC => None,
                    _ => { raise(); None },
                }
            }
        }
    }

    /// Loop over all the branches and issue a callback for each one.
    pub fn branch_foreach(&self, local: bool, remote: bool,
        op: &fn(name: &str, is_remote: bool) -> bool) -> bool
    {
        let flocal = if local { ext::GIT_BRANCH_LOCAL } else { 0 };
        let fremote = if remote { ext::GIT_BRANCH_REMOTE } else { 0 };
        let flags = flocal & fremote;
        unsafe {
            let payload: *c_void = cast::transmute(&op);
            let res = ext::git_branch_foreach(self.repo, flags, git_branch_foreach_cb, payload);
            match res {
                0 => true,
                ext::GIT_EUSER => false,
                _ => { raise(); false },
            }
        }
    }

    /// Return the name of the reference supporting the remote tracking branch,
    /// given the name of a local branch reference.
    pub fn upstream_name(&self, canonical_branch_name: &str) -> Option<~str>
    {
        let mut buf: [c_char, ..1024] = [0, ..1024];
        do canonical_branch_name.as_c_str |c_name| {
            do as_mut_buf(buf) |v, _len| {
                unsafe {
                    let res = ext::git_branch_upstream_name(v, 1024, self.repo, c_name);
                    if res >= 0 {
                        let ptr: *c_char = cast::transmute(v);
                        Some( from_c_str_len(ptr, res as uint) )
                    } else if res == ext::GIT_ENOTFOUND {
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
    pub fn git_branch_remote_name(&self, canonical_branch_name: &str)
        -> Result<~str, (~str, GitError)>
    {
        let mut buf: [c_char, ..1024] = [0, ..1024];
        do canonical_branch_name.as_c_str |c_name| {
            do as_mut_buf(buf) |v, _len| {
                unsafe {
                    let res = ext::git_branch_remote_name(v, 1024, self.repo, c_name);
                    if res >= 0 {
                        let ptr: *c_char = cast::transmute(v);
                        Ok( from_c_str_len(ptr, res as uint) )
                    } else {
                        Err( last_error() )
                    }
                }
            }
        }
    }

    /// Lookup a blob object from a repository.
    pub fn blob_lookup<'r>(&'r self, id: &OID) -> Option<~Blob<'r>>
    {
        let mut ptr: *ext::git_blob = ptr::null();
        unsafe {
            if ext::git_blob_lookup(&mut ptr, self.repo, id) == 0 {
                Some( ~Blob { blob: ptr, owner: self } )
            } else {
                None
            }
        }
    }

    /// Read a file from the working folder of a repository
    /// and write it to the Object Database as a loose blob
    pub fn blob_create_fromworkdir<'r>(&'r self, relative_path: &str)
        -> Result<~Blob<'r>, (~str, GitError)>
    {
        let mut oid = OID { id: [0, ..20] };
        let mut ptr: *ext::git_blob = ptr::null();
        do relative_path.as_c_str |c_path| {
            unsafe {
                if ext::git_blob_create_fromworkdir(&mut oid, self.repo, c_path) == 0 {
                    if ext::git_blob_lookup(&mut ptr, self.repo, &oid) != 0 {
                        fail!(~"blob lookup failure");
                    }
                    Ok( ~Blob { blob: ptr, owner: self } )
                } else {
                    Err( last_error() )
                }
            }
        }
    }

    /// Read a file from the filesystem and write its content
    /// to the Object Database as a loose blob
    pub fn blob_create_fromdisk<'r>(&'r self, relative_path: &str)
        -> Result<~Blob<'r>, (~str, GitError)>
    {
        let mut oid = OID { id: [0, ..20] };
        let mut ptr: *ext::git_blob = ptr::null();
        do relative_path.as_c_str |c_path| {
            unsafe {
                if ext::git_blob_create_fromdisk(&mut oid, self.repo, c_path) == 0 {
                    if ext::git_blob_lookup(&mut ptr, self.repo, &oid) != 0 {
                        fail!(~"blob lookup failure");
                    }
                    Ok( ~Blob { blob: ptr, owner: self } )
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
    pub fn blob_create_fromreader<'r>(&'r self, reader: &Reader, hintpath: Option<&str>)
        -> Result<~Blob<'r>, (~str, GitError)>
    {
        let mut oid = OID { id: [0, ..20] };
        unsafe {
            let c_path =
            match hintpath {
                None => ptr::null(),
                Some(pathref) => pathref.as_c_str(|ptr| {ptr}),
            };
            let payload: *c_void = cast::transmute(&reader);
            if (ext::git_blob_create_fromchunks(&mut oid, self.repo, c_path, git_blob_chunk_cb,
                    payload) == 0) {
                let mut ptr: *ext::git_blob = ptr::null();
                if ext::git_blob_lookup(&mut ptr, self.repo, &oid) != 0 {
                    fail!(~"blob lookup failure");
                }
                Ok( ~Blob { blob: ptr, owner: self } )
            } else {
                Err( last_error() )
            }
        }
    }

    /// Write an in-memory buffer to the ODB as a blob
    pub fn blob_create_frombuffer<'r>(&'r self, buffer: &[u8])
        -> Result<~Blob<'r>, (~str, GitError)>
    {
        let mut oid = OID { id: [0, ..20] };
        do as_imm_buf(buffer) |v, len| {
            unsafe {
                let buf:*c_void = cast::transmute(v);
                if ext::git_blob_create_frombuffer(&mut oid, self.repo, buf, len as u64) == 0 {
                    let mut ptr: *ext::git_blob = ptr::null();
                    if ext::git_blob_lookup(&mut ptr, self.repo, &oid) != 0 {
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
    pub fn commit<'r>(&'r self, update_ref: Option<&str>, author: &Signature,
            committer: &Signature, message_encoding: Option<&str>, message: &str, tree: &Tree,
            parents: &[~Commit<'r>]) -> OID
    {
        unsafe {
            let c_ref = 
            match update_ref {
                None => ptr::null(),
                Some(uref) => uref.as_c_str(|ptr| {ptr}),
            };
            let c_author = signature::to_c_sig(author);
            let c_committer = signature::to_c_sig(committer);
            let c_encoding =
            match message_encoding {
                None => ptr::null(),
                Some(enc) => enc.as_c_str(|ptr| {ptr}),
            };
            let c_message = message.as_c_str(|ptr| {ptr});
            let mut oid = OID { id: [0, .. 20] };
            let c_parents = do parents.map |p| { p.commit };
            do as_const_buf(c_parents) |parent_ptr, len| {
                let res = ext::git_commit_create(&mut oid, self.repo, c_ref,
                            &c_author, &c_committer, c_encoding, c_message, tree.tree,
                            len as c_int, parent_ptr);
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
    pub fn diff_tree_to_tree<'r>(&'r self, old_tree: Option<~Tree>, new_tree: Option<~Tree>,
            opts: &diff::DiffOption, notify_cb: &fn(DiffList, DiffDelta, ~str) -> WalkMode)
        -> Result<~DiffList, (~str, GitError)>
    {
        unsafe {
            let old_t = match old_tree {
                None => ptr::null(),
                Some(t) => t.tree,
            };

            let new_t = match new_tree {
                None => ptr::null(),
                Some(t) => t.tree,
            };

            let flags = do opts.flags.iter().fold(0u32) |flags, &f| {
                flags | (f as u32)
            };

            let pathspec = do opts.pathspec.map |path| {
                do path.as_c_str |c_path| { c_path }
            };

            let c_pathspec = ext::git_strarray {
                strings: std::vec::raw::to_ptr(pathspec),
                count: pathspec.len() as u64,
            };

            let c_opts = ext::git_diff_options {
                version: 1,     // GIT_DIFF_OPTIONS_VERSION
                flags: flags,
                context_lines: opts.context_lines,
                interhunk_lines: opts.interhunk_lines,
                old_prefix: do opts.old_prefix.as_c_str |c_pref| { c_pref },
                new_prefix: do opts.new_prefix.as_c_str |c_pref| { c_pref },
                pathspec: c_pathspec,
                max_size: opts.max_size,
                notify_cb: git_diff_notify_cb,
                notify_payload: cast::transmute(&notify_cb),
            };

            let mut diff_list: *ext::git_diff_list = ptr::null();

            if ext::git_diff_tree_to_tree(&mut diff_list, self.repo, old_t, new_t, &c_opts) == 0 {
                Ok( ~DiffList { difflist: diff_list } )
            } else {
                Err( last_error() )
            }
        }
    }
}

extern fn git_status_cb(path: *c_char, status_flags: c_uint, payload: *c_void) -> c_int
{
    unsafe {
        let op_ptr: *&fn(~str, c_uint) -> bool = cast::transmute(payload);
        let op = *op_ptr;
        let path_str = from_c_str(path);
        if op(path_str, status_flags) {
            0
        } else {
            1
        }
    }
}

extern fn git_blob_chunk_cb(content: *mut u8, max_length: size_t, payload: *&Reader) -> c_int
{
    let len = max_length as uint;
    unsafe {
        let reader = *payload;
        do mut_buf_as_slice(content, len) |v| {
            if reader.eof() {
                0
            } else {
                reader.read(v, len) as c_int
            }
        }
    }
}

extern fn git_branch_foreach_cb(branch_name: *c_char, branch_type: ext::git_branch_t,
    payload: *c_void) -> c_int
{
    unsafe {
        let op_ptr: *&fn(name: &str, is_remote: bool) -> bool = cast::transmute(payload);
        let op = *op_ptr;
        let branch_str = from_c_str(branch_name);
        let is_remote = (branch_type == ext::GIT_BRANCH_REMOTE);
        if op(branch_str, is_remote) {
            0
        } else {
            1
        }
    }
}

extern fn git_diff_notify_cb(diff_so_far: *ext::git_diff_list, delta_to_add: *DiffDelta,
    matched_pathspec: *c_char, payload: *c_void) -> c_int
{
    unsafe {
        let op_ptr: *&fn(DiffList, DiffDelta, ~str) -> bool = cast::transmute(payload);
        let op = *op_ptr;
        let difflist = DiffList { difflist: diff_so_far };
        let spec_str = from_c_str(matched_pathspec);
        op(difflist, *delta_to_add, spec_str) as c_int
    }
}

impl Drop for Repository {
    fn finalize(&self) {
        unsafe {
            ext::git_repository_free(self.repo);
        }
    }
}
