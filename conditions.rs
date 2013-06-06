pub use ext::{git_error_t, git_oid};
pub use super::*;

condition! {
    bad_repo: (~str, super::git_error_t) -> @super::Repository;
}

condition! {
    bad_path: (~str, super::git_error_t) -> ~str;
}

condition! {
    bad_ref: (~str, super::git_error_t) -> ~super::Reference;
}

condition! {
    bad_index: (~str, super::git_error_t) -> ~super::GitIndex;
}

condition! {
    bad_tree: (~str, super::git_error_t) -> ~super::Tree;
}

condition! {
    check_fail: (~str, super::git_error_t) -> bool;
}

condition! {
    checkout_fail: (~str, super::git_error_t) -> ();
}

condition! {
    index_fail: (~str, super::git_error_t) -> ();
}
