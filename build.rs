#![doc(html_no_source)]
use static_files::{resource::generate_resources_mapping, resource_dir, sets};
use std::{env, io, path::Path};

fn main() -> io::Result<()> {
    resource_dir("./tests").build()?;

    let out_dir = env::var("OUT_DIR").unwrap();
    let generated_filename = Path::new(&out_dir).join("generated_mapping.rs");
    generate_resources_mapping("./tests", None, generated_filename)?;

    sets::generate_resources_sets(
        "./tests",
        None,
        Path::new(&out_dir).join("generated_sets.rs"),
        "sets",
        "generate",
        &mut sets::SplitByCount::new(2),
    )?;

    Ok(())
}
