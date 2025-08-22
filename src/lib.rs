#![doc(test(no_crate_inject))]
#![doc = include_str!("../README.md")]
pub mod deps;
mod resource_files;
pub use resource_files::{ResourceFiles, UriSegmentError};
#[cfg(feature = "builtin-03")]
pub use static_files_03::*;
