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
echo "Hello, world" > static/hello
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
use actix_web_static_files::resource_dir;

fn main() {
    resource_dir("./static").build().unwrap();
}
```

Include generated code in `main.rs`:
```rust
use actix_web::{App, HttpServer};
use actix_web_static_files;

use std::collections::HashMap;

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

Run the server:
```bash
cargo run
```

Request the resource:
```bash
$ curl -v http://localhost:8080/static/hello
*   Trying 127.0.0.1:8080...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 8080 (#0)
> GET /static/hello HTTP/1.1
> Host: localhost:8080
> User-Agent: curl/7.65.3
> Accept: */*
> 
* Mark bundle as not supporting multiuse
< HTTP/1.1 200 OK
< content-length: 13
< date: Tue, 06 Aug 2019 13:36:50 GMT
< 
Hello, world
* Connection #0 to host localhost left intact
```