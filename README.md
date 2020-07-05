# actix-web static files as resources support

## Legal

Dual-licensed under `MIT` or the [UNLICENSE](http://unlicense.org/).

## Features

- Embed static resources in executuble
- Serve static resources as directory in `actix-web`
- Install dependencies with [npm](https://npmjs.org) package manager
- Run custom `npm` run commands (such as [webpack](https://webpack.js.org/))

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
actix-web-static-files = "2.0"

[build-dependencies]
actix-web-static-files = "2.0"
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

```rust
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

```js
const path = require('path');
const { CleanWebpackPlugin } = require('clean-webpack-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin');

module.exports = {
  entry: './src/index.js',
  plugins: [
    new CleanWebpackPlugin(),
    new HtmlWebpackPlugin({
      title: 'actix-web-static-files WebPack',
    }),
  ],
  output: {
    filename: 'main.js',
    path: path.resolve(__dirname, 'dist'),
  },
};
```

Add `web/src/index.js`:

```js
import _ from 'lodash';

function component() {
  const element = document.createElement('div');

  element.innerHTML = _.join(['Hello', 'webpack'], ' ');

  return element;
}

document.body.appendChild(component());
```

Modify `web/package.json` by adding "scripts" sections:

```json
{
  "dependencies": {
    "lodash": "^4.17.15"
  },
  "devDependencies": {
    "clean-webpack-plugin": "^3.0.0",
    "html-webpack-plugin": "^3.2.0",
    "webpack": "^4.41.5",
    "webpack-cli": "^3.3.10"
  },
  "scripts": {
    "build": "webpack"
  }
}
```

Add to `Cargo.toml` dependency to `actix-web-static-files` as in the first use case.

Add build script to `Cargo.toml` as in the first use case.

Add `build.rs` with call to bundle resources:

```rust
use actix_web_static_files::NpmBuild;

fn main() {
    NpmBuild::new("./web")
        .executable("yarn")
        .install().unwrap()
        .run("build").unwrap()
        .target("./web/dist")
        .to_resource_dir()
        .build().unwrap();
}
```

Include generated code in `src/main.rs`:

```rust
use actix_web::{App, HttpServer};
use actix_web_static_files;

use std::collections::HashMap;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        let generated = generate();
        App::new().service(actix_web_static_files::ResourceFiles::new(
            "/", generated,
        ))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

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
