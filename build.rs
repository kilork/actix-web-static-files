#![doc(html_no_source)]
include!("src/impl.rs");
fn main() {
    resource_dir("./tests").build().unwrap();
}
