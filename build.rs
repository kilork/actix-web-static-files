#![doc(html_no_source)]
include!("impl.rs");
fn main() {
    resource_dir("./tests").build().unwrap();
}
