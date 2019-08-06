# actix-web static files as resources support

## Legal

Dual-licensed under `MIT` or the [UNLICENSE](http://unlicense.org/).

## Features

- Embed static resources in end binary
- Serve static resources as directory in `actix-web`

## Usage

Create folder with static resources in your project (for example `static`):
```bash
cd project_dir
mkdir static
```

Add to `Cargo.toml` dependency to `actix-web-static-files`:
```toml
[dependencies]
actix-web-static-files = "0.1"

[build-dependencies]
actix-web-static-files = "0.1"
```

Add build script to `Cargo.toml`:
```toml
[package]
build = "build.rs"
```

Add `build.rs` with call to bundle resources:
```rust
use actix_web_static_files;
use std::{env, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let dest_path = Path::new(&out_dir).join("generated.rs");
    actix_web_static_files::generate_resources(
        "./static",
        None,
        &dest_path,
        "generate",
    )
    .unwrap();
}
```

Include generated code in `main.rs`:
```rust
use actix_web::web;
use actix_web::{App, HttpResponse, HttpServer};
use actix_web_static_files;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

fn main() {
    HttpServer::new(move || {
        let generated = generate();
        App::new()
        .service(actix_web_static_files::ResourceFiles::new(
            "/static",
            generated,
        ))
    })
    .bind("127.0.0.1:8080").unwrap()
    .run().unwrap();
}
```