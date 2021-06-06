#![doc(test(no_crate_inject))]
/*!
{{ replace ( render ( read_to_str "templates/README.md" ) ) "```rust" "```rust#ignore" }}*/

pub mod deps;
mod resource_files;
pub use resource_files::{ResourceFiles, UriSegmentError};
