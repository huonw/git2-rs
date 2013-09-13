#[link(name = "git2",
       vers = "0.1-pre",
       url = "https://github.com/kimhyunkang/git2-rs")];

#[comment = "libgit2 binding for Rust"];
#[license = "MIT"];

#[crate_type = "lib"];

use std::cast;

pub use tree::{Tree, TreeBuilder, TreeEntry};
pub use commit::Commit;
pub use blob::Blob;
pub use repository::Repository;
pub use reference::Reference;
pub use git_index::GitIndex;

pub mod ffi;
pub mod repository;
pub mod reference;
pub mod git_index;
pub mod tree;
pub mod blob;
pub mod commit;
pub mod signature;
pub mod oid;
pub mod diff;

#[doc(hidden)]
pub mod linkhack {
    #[link_args="-lgit2"]
    extern {
    }
}

condition! {
    git_error: (~str, super::GitError) -> ();
}

pub unsafe fn raise() {
    git_error::cond.raise(last_error())
}

#[fixed_stack_segment]
pub unsafe fn last_error() -> (~str, GitError) {
    let err = ffi::giterr_last();
    let message = std::str::raw::from_c_str((*err).message as *i8);
    let klass = (*err).klass;
    (message, cast::transmute(klass as u64))
}

/** Error classes */
#[deriving(Eq,ToStr,Clone)]
pub enum GitError {
    GITERR_NOMEMORY,
    GITERR_OS,
    GITERR_INVALID,
    GITERR_REFERENCE,
    GITERR_ZLIB,
    GITERR_REPOSITORY,
    GITERR_CONFIG,
    GITERR_REGEX,
    GITERR_ODB,
    GITERR_INDEX,
    GITERR_OBJECT,
    GITERR_NET,
    GITERR_TAG,
    GITERR_TREE,
    GITERR_INDEXER,
    GITERR_SSL,
    GITERR_SUBMODULE,
    GITERR_THREAD,
    GITERR_STASH,
    GITERR_CHECKOUT,
    GITERR_FETCHHEAD,
    GITERR_MERGE,
}

pub enum WalkMode {
    WalkSkip = 1,
    WalkPass = 0,
    WalkStop = -1,
}

pub enum DiffDelta {
    GIT_DELTA_UNMODIFIED = 0, // no changes
    GIT_DELTA_ADDED = 1,      // entry does not exist in old version
    GIT_DELTA_DELETED = 2,    // entry does not exist in new version
    GIT_DELTA_MODIFIED = 3,   // entry content changed between old and new
    GIT_DELTA_RENAMED = 4,    // entry was renamed between old and new
    GIT_DELTA_COPIED = 5,     // entry was copied from another old entry
    GIT_DELTA_IGNORED = 6,    // entry is ignored item in workdir
    GIT_DELTA_UNTRACKED = 7,  // entry is untracked item in workdir
    GIT_DELTA_TYPECHANGE = 8, // type of entry changed between old and new
}

pub struct DiffList {
    priv difflist: *mut ffi::git_diff_list,
}

#[deriving(Clone)]
pub struct Time {
    time: i64,      /* time in seconds from epoch */
    offset: int,    /* timezone offset, in minutes */
}

#[deriving(Eq, Clone)]
pub struct Signature {
    name: ~str,
    email: ~str,
    when: Time,
}

pub struct OID {
    id: [std::libc::c_char, ..20],
}

/// Status flags for a single file.
///
/// A combination of these values will be returned to indicate the status of a file.
/// Status compares the working directory, the index, and the current HEAD of the repository.
/// The `index` set of flags represents the status of file in the index relative to the HEAD,
/// and the `wt` set of flags represent the status of the file in the working directory
/// relative to the index
#[deriving(Clone,Eq)]
pub struct Status {
    index_new: bool,
    index_modified: bool,
    index_deleted: bool,
    index_renamed: bool,
    index_typechange: bool,

    wt_new: bool,
    wt_modified: bool,
    wt_deleted: bool,
    wt_typechange: bool,

    ignored: bool,
}

impl Status {
    /// set every flags to false
    pub fn new() -> Status {
        Status {
            index_new: false,
            index_modified: false,
            index_deleted: false,
            index_renamed: false,
            index_typechange: false,

            wt_new: false,
            wt_modified: false,
            wt_deleted: false,
            wt_typechange: false,

            ignored: false,
        }
    }
}

/// Valid modes for index and tree entries.
pub enum FileMode {
    GIT_FILEMODE_NEW                    = 0x0000,   // 0000000
    GIT_FILEMODE_TREE                   = 0x4000,   // 0040000
    GIT_FILEMODE_BLOB                   = 0x81a4,   // 0100644
    GIT_FILEMODE_BLOB_EXECUTABLE        = 0x81ed,   // 0100755
    GIT_FILEMODE_LINK                   = 0xa000,   // 0120000
    GIT_FILEMODE_COMMIT                 = 0xe000,   // 0160000
}

/// Basic type (loose or packed) of any Git object.
pub enum OType {
    GIT_OBJ_ANY = -2,       // Object can be any of the following
    GIT_OBJ_BAD = -1,       // Object is invalid.
    GIT_OBJ__EXT1 = 0,      // Reserved for future use.
    GIT_OBJ_COMMIT = 1,     // A commit object.
    GIT_OBJ_TREE = 2,       // A tree (directory listing) object.
    GIT_OBJ_BLOB = 3,       // A file revision object.
    GIT_OBJ_TAG = 4,        // An annotated tag object.
    GIT_OBJ__EXT2 = 5,      // Reserved for future use.
    GIT_OBJ_OFS_DELTA = 6,  // A delta, base is given by an offset.
    GIT_OBJ_REF_DELTA = 7,  // A delta, base is given by object id.
}


// FIXME: there should be better ways to do this...
// if you call this library in multiple tasks,
// this function must be called before calling any other functions in library
#[fixed_stack_segment]
pub fn threads_init() {
    unsafe {
        ffi::git_threads_init();
    }
}

// if you call this library in multiple tasks,
// this function must be called before shutting down the library
#[fixed_stack_segment]
pub fn threads_shutdown() {
    unsafe {
        ffi::git_threads_shutdown();
    }
}
