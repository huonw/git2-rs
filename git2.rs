#[link(name = "git2",
       vers = "0.1-pre",
       url = "https://github.com/kimhyunkang/git2-rs")];

#[comment = "libgit2 binding for Rust"];
#[license = "MIT"];

#[crate_type = "lib"];

pub mod error;
pub mod ext;
pub mod types;
pub mod repository;

pub type Repository = types::Repository;
pub type GitError = types::GitError;

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
