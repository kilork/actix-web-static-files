#![doc(test(no_crate_inject))]
/*!
# actix-web static files as resources support

<!-- vscode-markdown-toc -->
- [actix-web static files as resources support](#actix-web-static-files-as-resources-support)
  - [<a name='Legal'></a>Legal](#legal)
  - [<a name='Features'></a>Features](#features)
  - [<a name='Usage'></a>Usage](#usage)
    - [<a name='usecase1'></a>Use-case 1: Static resources folder](#use-case-1-static-resources-folder)
    - [<a name='usecase2'></a>Use-case 2: package.json - npm managed folder](#use-case-2-packagejson---npm-managed-folder)
    - [<a name='usecase3'></a>Use-case 3: package.json - WebPack usage](#use-case-3-packagejson---webpack-usage)
    - [<a name='usecase4'></a>Use-case 4: yarn package manager](#use-case-4-yarn-package-manager)
    - [<a name='usecase5'></a>Use-case 5: Angular-like applications](#use-case-5-angular-like-applications)

<!-- vscode-markdown-toc-config
    numbering=false
    autoSave=true
    /vscode-markdown-toc-config -->
<!-- /vscode-markdown-toc -->

## <a name='Legal'></a>Legal

Dual-licensed under `MIT` or the [Unlicense](http://unlicense.org/).

## <a name='Features'></a>Features

- Embed static resources in single self-contained executable
- Serve static resources in `actix-web`
- Install dependencies with [npm](https://npmjs.org) package manager
- Run custom `npm` run commands (such as [webpack](https://webpack.js.org/))
- Support for npm-like package managers ([yarn](https://yarnpkg.com/))
- Support for angular-like routers

## <a name='Usage'></a>Usage

### <a name='usecase1'></a>Use-case 1: Static resources folder

Create folder with static resources in your project (for example `static`):

```bash
cd project_dir
mkdir static
echo "<p>Hello, world\!</p>" > static/index.html
```

Add to `Cargo.toml` dependencies related to `actix-web-static-files`:

```toml
[dependencies]
actix-web = "4.0"
actix-web-static-files = "4.0"
static-files = "0.2.1"

[build-dependencies]
static-files = "0.2.1"
```

Add `build.rs` with call to bundle resources:

```rust#ignore
use static_files::resource_dir;

fn main() -> std::io::Result<()> {
    resource_dir("./static").build()
}
```

Include generated code in `src/main.rs`:

```rust#ignore
use actix_web::{App, HttpServer};
use actix_web_static_files::ResourceFiles;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        let generated = generate();
        App::new().service(ResourceFiles::new("/", generated))
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

- [Static resources folder with index.html example](https://github.com/kilork/actix-web-static-files-examples/tree/v4.0/resource-dir)
- [Another example with same resources but using own defined function](https://github.com/kilork/actix-web-static-files-examples/tree/v4.0/generate-resources-mapping)


### <a name='usecase2'></a>Use-case 2: package.json - npm managed folder

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
use static_files::npm_resource_dir;

fn main() -> std::io::Result<()> {
    npm_resource_dir("./static_packages")?.build()
}
```

Include generated code in `main.rs` same way as in the first use-case.

Reference resources in your `HTML` (`static/index.html`):

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

### <a name='usecase3'></a>Use-case 3: package.json - WebPack usage

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
    path: path.resolve(__dirname, 'dist', 'bundle'),
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
    "lodash": "^4.17.21"
  },
  "devDependencies": {
    "clean-webpack-plugin": "^3.0.0",
    "html-webpack-plugin": "^5.2.0",
    "webpack": "^5.24.2",
    "webpack-cli": "^4.5.0"
  },
  "scripts": {
    "build": "webpack"
  }
}
```

Add to `Cargo.toml` dependency to `actix-web-static-files` as in the first use case.

Add `build.rs` with call to bundle resources:

```rust#ignore
use static_files::NpmBuild;

fn main() -> std::io::Result<()> {
    NpmBuild::new("web")
        .install()?
        .run("build")?
        .target("web/dist/bundle")
        .change_detection()
        .to_resource_dir()
        .build()
}
```

Include generated code in `src/main.rs`:

```rust#ignore
use actix_web::{App, HttpServer};
use actix_web_static_files;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        let generated = generate();
        App::new().service(actix_web_static_files::ResourceFiles::new("/", generated))
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

See also:

- [WebPack Example](https://github.com/kilork/actix-web-static-files-examples/tree/v4.0/webpack)

### <a name='usecase4'></a>Use-case 4: yarn package manager

We can use another package manager instead of `npm`. For example, to use [yarn](https://yarnpkg.com/) just add `.executable("yarn")` to `NpmBuild` call:

```rust#ignore
use static_files::NpmBuild;

fn main() -> std::io::Result<()> {
    NpmBuild::new("web")
        .executable("yarn")
        .install()?
        .run("build")?
        .target("web/dist/bundle")
        .change_detection()
        .to_resource_dir()
        .build()
}
```

See also:

- [Yarn WebPack Example](https://github.com/kilork/actix-web-static-files-examples/tree/v4.0/yarn-webpack)

### <a name='usecase5'></a>Use-case 5: Angular-like applications

If you are using Angular as frontend, you may want to resolve all not found calls via `index.html` of frontend app. To do this just call method `resolve_not_found_to_root` after resource creation.

```rust#ignore
use actix_web::{middleware::Logger, App, HttpServer};
#[cfg(feature = "ui")]
use actix_web_static_files;

#[cfg(feature = "ui")]
use angular_example_frontend::generate;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    HttpServer::new(move || {
        let mut app = App::new().wrap(Logger::default());
        #[cfg(feature = "ui")]
        {
            let generated = generate();
            app = app.service(
                actix_web_static_files::ResourceFiles::new("/", generated)
                    .resolve_not_found_to_root(),
            );
        }
        app
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

Remember to place you static resource route after all other routes in this case.

You can check the complete example [Angular Router Sample](https://github.com/kilork/actix-web-static-files-example-angular-router/tree/v4.0).
*/

pub mod deps;
mod resource_files;
pub use resource_files::{ResourceFiles, UriSegmentError};
