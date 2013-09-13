use super::{OID};
use std::cast;
use std::vec::raw::buf_as_slice;
use ffi;
use repository::Repository;

pub struct Blob<'self> {
    blob: *mut ffi::git_blob,
    owner: &'self Repository,
}

impl<'self> Blob<'self> {
    /// get the id of the blob
    #[fixed_stack_segment]
    pub fn id<'r>(&self) -> &'r OID
    {
        unsafe {
            // OID pointer returned by git_blob_id is const pointer
            // so it's safe to use as long as self is alive
            cast::transmute(ffi::git_blob_id(self.blob as *ffi::git_blob))
        }
    }

    ///
    /// Get a read-only buffer with the raw content of a blob.
    ///
    /// A reference to the raw content of a blob is transferred to closure
    #[fixed_stack_segment]
    pub fn rawcontent_as_slice<T>(&self, f: &fn(v: &[u8]) -> T) -> T
    {
        unsafe {
            let ptr:*u8 = cast::transmute(ffi::git_blob_rawcontent(self.blob as *ffi::git_blob));
            let size = ffi::git_blob_rawsize(self.blob as *ffi::git_blob);
            if(size < 0) {
                fail!(~"negative blob size")
            }
            buf_as_slice(ptr, size as uint, f)
        }
    }

    /// Determine if the blob content is most certainly binary or not.
    ///
    /// The heuristic used to guess if a file is binary is taken from core git:
    /// Searching for NUL bytes and looking for a reasonable ratio of printable
    /// to non-printable characters among the first 4000 bytes.
    #[fixed_stack_segment]
    pub fn is_binary(&self) -> bool
    {
        unsafe {
            ffi::git_blob_is_binary(self.blob) != 0
        }
    }
}

#[unsafe_destructor]
impl<'self> Drop for Blob<'self> {
    #[fixed_stack_segment]
    fn drop(&self) {
        unsafe {
            ffi::git_blob_free(self.blob);
        }
    }
}
