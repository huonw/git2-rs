#[link(name = "git2",
       vers = "0.1-pre",
       url = "https://github.com/kimhyunkang/git2-rs")];

#[comment = "libgit2 binding for Rust"];
#[license = "MIT"];

#[crate_type = "lib"];

extern mod std;

pub mod ext;
pub mod conditions;
pub mod repository;
pub mod reference;
pub mod index;
pub mod tree;
pub mod commit;
pub mod signature;
pub mod oid;

pub struct Repository {
    pub repo: *ext::git_repository,
}

pub struct Reference {
    pub c_ref: *ext::git_reference,
    pub repo_ptr: @mut Repository,
}

pub struct GitIndex {
    pub index: *ext::git_index,
    pub owner: @mut Repository,
}

pub struct Tree {
    pub tree: *ext::git_tree,
    pub owner: @mut Repository,
}

pub struct TreeEntry {
    pub tree_entry: *ext::git_tree_entry,
    pub owned: bool,
}

pub struct TreeBuilder {
    pub bld: *ext::git_treebuilder,
}

pub struct Commit {
    pub commit: *ext::git_commit,
    pub owner: @mut Repository,
}

pub struct Signature {
    pub name: ~str,
    pub email: ~str,
    pub when: std::time::Tm,
}

pub struct OID {
    pub id: [core::libc::c_char, ..20],
}

/// Status flags for a single file.
///
/// A combination of these values will be returned to indicate the status of a file.
/// Status compares the working directory, the index, and the current HEAD of the repository.
/// The `index` set of flags represents the status of file in the index relative to the HEAD,
/// and the `wt` set of flags represent the status of the file in the working directory
/// relative to the index.
pub struct Status {
    pub index_new: bool,
    pub index_modified: bool,
    pub index_deleted: bool,
    pub index_renamed: bool,
    pub index_typechange: bool,

    pub wt_new: bool,
    pub wt_modified: bool,
    pub wt_deleted: bool,
    pub wt_typechange: bool,

    pub ignored: bool,
}

/// Valid modes for index and tree entries.
pub enum FileMode {
	GIT_FILEMODE_NEW					= 0000000,
	GIT_FILEMODE_TREE					= 0040000,
	GIT_FILEMODE_BLOB					= 0100644,
	GIT_FILEMODE_BLOB_EXECUTABLE		= 0100755,
	GIT_FILEMODE_LINK					= 0120000,
	GIT_FILEMODE_COMMIT					= 0160000,
}

/// Basic type (loose or packed) of any Git object.
pub enum OType {
	GIT_OBJ_ANY = -2,		// Object can be any of the following
	GIT_OBJ_BAD = -1,		// Object is invalid.
	GIT_OBJ__EXT1 = 0,		// Reserved for future use.
	GIT_OBJ_COMMIT = 1,		// A commit object.
	GIT_OBJ_TREE = 2,		// A tree (directory listing) object.
	GIT_OBJ_BLOB = 3,		// A file revision object.
	GIT_OBJ_TAG = 4,		// An annotated tag object.
	GIT_OBJ__EXT2 = 5,		// Reserved for future use.
	GIT_OBJ_OFS_DELTA = 6,  // A delta, base is given by an offset.
	GIT_OBJ_REF_DELTA = 7,  // A delta, base is given by object id.
}


// FIXME: there should be better ways to do this...
// if you call this library in multiple tasks,
// this function must be called before calling any other functions in library
pub fn threads_init() {
    unsafe {
        ext::git_threads_init();
    }
}

// if you call this library in multiple tasks,
// this function must be called before shutting down the library
pub fn threads_shutdown() {
    unsafe {
        ext::git_threads_shutdown();
    }
}
