#![doc(test(no_crate_inject))]
/*!
{{ render (replace (read_to_str "templates/README.md") "```rust" "```rust#ignore") }}*/

pub mod deps;
mod resource_files;
pub use resource_files::{ResourceFiles, UriSegmentError};
