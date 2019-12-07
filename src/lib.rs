#![doc(test(no_crate_inject))]
/*!
# actix-web static files as resources support

## Legal

Dual-licensed under `MIT` or the [UNLICENSE](http://unlicense.org/).

## Features

- Embed static resources in end binary
- Serve static resources as directory in `actix-web`
- Install package manager (npm) dependincies

## Usage

### Use-case #1: Static resources folder

Create folder with static resources in your project (for example `static`):

```bash
cd project_dir
mkdir static
echo "Hello, world" > static/hello
```

Add to `Cargo.toml` dependency to `actix-web-static-files`:

```toml
[dependencies]
actix-web-static-files = "0.3.0-alpha.2"

[build-dependencies]
actix-web-static-files = "0.3.0-alpha.2"
```

Add build script to `Cargo.toml`:

```toml
[package]
build = "build.rs"
```

Add `build.rs` with call to bundle resources:

```rust#ignore
use actix_web_static_files::resource_dir;

fn main() {
    resource_dir("./static").build().unwrap();
}
```

Include generated code in `main.rs`:

```rust#ignore
use actix_web::{App, HttpServer};
use actix_web_static_files;

use std::collections::HashMap;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        let generated = generate();
        App::new().service(actix_web_static_files::ResourceFiles::new(
            "/static", generated,
        ))
    })
    .bind("127.0.0.1:8080")?
    .start()
    .await
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
>
* Mark bundle as not supporting multiuse
< HTTP/1.1 200 OK
< content-length: 13
< date: Tue, 06 Aug 2019 13:36:50 GMT
<
Hello, world
* Connection #0 to host localhost left intact
```

### Use-case #2: package.json - npm managed folder

Create folder with static resources in your project (for example `static`):

```bash
cd project_dir
mkdir static_packages
cd static_packages
echo '{}' > package.json
# install your npm dependencies (here we use fontawesome as an example)
npm install --save-dev @fortawesome/fontawesome-free
```

Add generated folder to ignore file of your version control system (here: git):

```bash
cd project_dir
echo "static_packages/node_modules" >> .gitignore
```

Add `dependencies` and `build-dependencies` in `Cargo.toml` same way as in the first use-case.

Add `build.rs` with call to bundle resources:

```rust#ignore
use actix_web_static_files::npm_resource_dir;

fn main() {
    npm_resource_dir("./static_packages").unwrap().build().unwrap();
}
```

Include generated code in `main.rs` same way as in the first use-case.

Reference resources in your `HTML`:

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
    <link rel="stylesheet" href="/static/@fortawesome/fontawesome-free/css/all.css">
    <script defer src="/static/@fortawesome/fontawesome-free/js/all.js"></script>
    <title>Hi</title>
</head>
<body>
    <i class="fas fa-thumbs-up"></i>
</body>
</html>
```
*/
include!("impl.rs");
