# actix-web static files as resources support

## Legal

Dual-licensed under `MIT` or the [UNLICENSE](http://unlicense.org/).

## Features

- Embed static resources in single self-contained executuble
- Serve static resources in `actix-web`
- Install dependencies with [npm](https://npmjs.org) package manager
- Run custom `npm` run commands (such as [webpack](https://webpack.js.org/))
- Support for npm-like package managers ([yarn](https://yarnpkg.com/))
- Support for angular-like routers

## Usage

### Use-case #1: Static resources folder

Create folder with static resources in your project (for example `static`):

```bash
cd project_dir
mkdir static
echo "<p>Hello, world\!</p>" > static/index.html
```

Add to `Cargo.toml` dependencies related to `actix-web-static-files`:

```toml
{{ from "[dependencies]" (http_get "https://raw.githubusercontent.com/kilork/actix-web-static-files-examples/kilork/issue1/resource-dir/Cargo.toml") }}```

Add `build.rs` with call to bundle resources:

```rust
{{ http_get "https://raw.githubusercontent.com/kilork/actix-web-static-files-examples/kilork/issue1/resource-dir/build.rs" }}```

Include generated code in `src/main.rs`:

```rust
{{ http_get "https://raw.githubusercontent.com/kilork/actix-web-static-files-examples/kilork/issue1/resource-dir/src/main.rs" }}```

Run the server:

```bash
cargo run
```

Request the resource:

```bash
$ curl -v http://localhost:8080/
*   Trying ::1...
* TCP_NODELAY set
* Connection failed
* connect to ::1 port 8080 failed: Connection refused
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 8080 (#0)
> GET / HTTP/1.1
> Host: localhost:8080
> User-Agent: curl/7.64.1
> 
< HTTP/1.1 200 OK
< content-length: 20
< content-type: text/html
< etag: "14:606a2226"
< date: Sun, 23 May 2021 19:46:42 GMT
< 
* Connection #0 to host localhost left intact
<p>Hello, world!</p>* Closing connection 0
```

See also:

- [Static resources folder with index.html example](https://github.com/kilork/actix-web-static-files-examples/tree/v3.1/resource-dir)
- [Another example with same resources but using own defined function](https://github.com/kilork/actix-web-static-files-examples/tree/v3.1/generate-resources-mapping)


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

{{ codeblock "rust" ( http_get "https://raw.githubusercontent.com/kilork/actix-web-static-files-examples/kilork/issue1/npm-resource-dir/build.rs" ) }}

Include generated code in `main.rs` same way as in the first use-case.

Reference resources in your `HTML` (`static/index.html`):

{{ codeblock "html" ( http_get "https://raw.githubusercontent.com/kilork/actix-web-static-files-examples/kilork/issue1/npm-resource-dir/static/index.html" ) }}

### Use-case #3: package.json - WebPack usage

Create folder with static resources in your project (for example `web`), install required packages and webpack:

```bash
cd project_dir
mkdir -p web/src
cd web
echo -e "node_modules\ndist" > .gitignore
echo '{}' > package.json


# install lodash for usage in example
npm install --save lodash

# install webpack npm dependencies
npm install webpack webpack-cli html-webpack-plugin clean-webpack-plugin --save-dev
```

Add `web/webpack.config.js`:

{{ codeblock "js" ( http_get "https://raw.githubusercontent.com/kilork/actix-web-static-files-examples/kilork/issue1/webpack/web/webpack.config.js" ) }}

Add `web/src/index.js`:

{{ codeblock "js" ( http_get "https://raw.githubusercontent.com/kilork/actix-web-static-files-examples/kilork/issue1/webpack/web/src/index.js" ) }}

Modify `web/package.json` by adding "scripts" sections:

{{ codeblock "json" ( http_get "https://raw.githubusercontent.com/kilork/actix-web-static-files-examples/kilork/issue1/webpack/web/package.json" ) }}

Add to `Cargo.toml` dependency to `actix-web-static-files` as in the first use case.

Add `build.rs` with call to bundle resources:

{{ codeblock "rust" ( http_get "https://raw.githubusercontent.com/kilork/actix-web-static-files-examples/kilork/issue1/webpack/build.rs" ) }}

Include generated code in `src/main.rs`:

{{ codeblock "rust" ( http_get "https://raw.githubusercontent.com/kilork/actix-web-static-files-examples/kilork/issue1/webpack/src/main.rs" ) }}

Run the server:

```bash
cargo run
```

Request the resource:

```bash
$ curl -v http://localhost:8080
*   Trying ::1...
* TCP_NODELAY set
* Connection failed
* connect to ::1 port 8080 failed: Connection refused
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 8080 (#0)
> GET / HTTP/1.1
> Host: localhost:8080
> User-Agent: curl/7.64.1
>
< HTTP/1.1 200 OK
< content-length: 199
< content-type: text/html
< etag: "c7:5e403845"
< date: Sun, 09 Feb 2020 16:51:45 GMT
<
<!DOCTYPE html>
<html>
  <head>
    <meta charset="UTF-8">
    <title>actix-web-static-files WebPack</title>
  </head>
  <body>
  <script type="text/javascript" src="main.js"></script></body>
```

See also:

- [WebPack Example](https://github.com/kilork/actix-web-static-files-examples/tree/v3.1/webpack)

### Use-case #4: yarn package manager

We can use another package manager instead of `npm`. For example, to use [yarn](https://yarnpkg.com/) just add `.executable("yarn")` to `NpmBuild` call:

```rust
use actix_web_static_files::NpmBuild;

fn main() {
    NpmBuild::new("./web")
        .executable("yarn")
        .install().unwrap()
        .run("build").unwrap()
        .target("./web/dist")
        .change_detection()
        .to_resource_dir()
        .build().unwrap();
}
```

### Use-case #5: Angular-like applications

If you are using Angular as frontend, you may want to resolve all not found calls via `index.html` of frontend app. To do this just call method `resolve_not_found_to_root` after resource creation.

```rust
use actix_web::{App, HttpServer};
use actix_web_static_files;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        let generated = generate();
        App::new().service(actix_web_static_files::ResourceFiles::new(
            "/", generated,
        ).resolve_not_found_to_root(),)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

Remember to place you static resources route after all other routes in this case.

You can check the complete example [Angular Router Sample](https://github.com/kilork/actix-web-static-files-example-angular-router).
