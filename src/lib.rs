#![doc(test(no_crate_inject))]
#![doc = include_str!("../README.md")]
pub mod deps;
mod resource_files;
pub use resource_files::{ResourceFile, ResourceFiles, ResourceFilesCollection, UriSegmentError};
#[cfg(feature = "static-files-03")]
mod static_files_03;
