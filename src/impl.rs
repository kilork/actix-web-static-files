use actix_service::{Service, ServiceFactory};
use actix_web::{
    dev::{AppService, HttpServiceFactory, ResourceDef, ServiceRequest, ServiceResponse},
    error::Error,
    http::{header, Method, StatusCode},
    HttpMessage, HttpRequest, HttpResponse, ResponseError,
};
use derive_more::{Display, Error};
use futures::future::{ok, FutureExt, LocalBoxFuture, Ready};
use path_slash::PathExt;
use std::{
    collections::HashMap,
    env,
    fs::{self, File, Metadata},
    io::{self, Write},
    ops::Deref,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    task::{Context, Poll},
    time::SystemTime,
};

/// Static files resource.
pub struct Resource {
    pub data: &'static [u8],
    pub modified: u64,
    pub mime_type: &'static str,
}

#[inline]
pub fn new_resource(data: &'static [u8], modified: u64, mime_type: &'static str) -> Resource {
    Resource {
        data,
        modified,
        mime_type,
    }
}

/// Static resource files handling
///
/// `ResourceFiles` service must be registered with `App::service` method.
///
/// ```rust
/// use std::collections::HashMap;
///
/// use actix_web::App;
///
/// fn main() {
/// // serve root directory with default options:
/// // - resolve index.html
///     let files: HashMap<&'static str, actix_web_static_files::Resource> = HashMap::new();
///     let app = App::new()
///         .service(actix_web_static_files::ResourceFiles::new("/", files));
/// // or subpath with additional option to not resolve index.html
///     let files: HashMap<&'static str, actix_web_static_files::Resource> = HashMap::new();
///     let app = App::new()
///         .service(actix_web_static_files::ResourceFiles::new("/imgs", files)
///             .do_not_resolve_defaults());
/// }
/// ```
pub struct ResourceFiles {
    not_resolve_defaults: bool,
    not_found_resolves_to: Option<String>,
    inner: Rc<ResourceFilesInner>,
}

pub struct ResourceFilesInner {
    path: String,
    files: HashMap<&'static str, Resource>,
}

const INDEX_HTML: &str = "index.html";

impl ResourceFiles {
    pub fn new(path: &str, files: HashMap<&'static str, Resource>) -> Self {
        let inner = ResourceFilesInner {
            path: path.into(),
            files,
        };
        Self {
            inner: Rc::new(inner),
            not_resolve_defaults: false,
            not_found_resolves_to: None,
        }
    }

    /// By default trying to resolve '.../' to '.../index.html' if it exists.
    /// Turn off this resolution by calling this function.
    pub fn do_not_resolve_defaults(mut self) -> Self {
        self.not_resolve_defaults = true;
        self
    }

    /// Resolves not found references to this path.
    ///
    /// This can be useful for angular-like applications.
    pub fn resolve_not_found_to<S: ToString>(mut self, path: S) -> Self {
        self.not_found_resolves_to = Some(path.to_string());
        self
    }

    /// Resolves not found references to root path.
    ///
    /// This can be useful for angular-like applications.
    pub fn resolve_not_found_to_root(self) -> Self {
        self.resolve_not_found_to(INDEX_HTML)
    }
}

impl Deref for ResourceFiles {
    type Target = ResourceFilesInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl HttpServiceFactory for ResourceFiles {
    fn register(self, config: &mut AppService) {
        let rdef = if config.is_root() {
            ResourceDef::root_prefix(&self.path)
        } else {
            ResourceDef::prefix(&self.path)
        };
        config.register_service(rdef, None, self, None)
    }
}

impl ServiceFactory for ResourceFiles {
    type Config = ();
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = Error;
    type Service = ResourceFilesService;
    type InitError = ();
    type Future = LocalBoxFuture<'static, Result<Self::Service, Self::InitError>>;

    fn new_service(&self, _: ()) -> Self::Future {
        ok(ResourceFilesService {
            resolve_defaults: !self.not_resolve_defaults,
            not_found_resolves_to: self.not_found_resolves_to.clone(),
            inner: self.inner.clone(),
        })
        .boxed_local()
    }
}

pub struct ResourceFilesService {
    resolve_defaults: bool,
    not_found_resolves_to: Option<String>,
    inner: Rc<ResourceFilesInner>,
}

impl Deref for ResourceFilesService {
    type Target = ResourceFilesInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> Service for ResourceFilesService {
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = Error;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        match *req.method() {
            Method::HEAD | Method::GET => (),
            _ => {
                return ok(ServiceResponse::new(
                    req.into_parts().0,
                    HttpResponse::MethodNotAllowed()
                        .header(header::CONTENT_TYPE, "text/plain")
                        .header(header::ALLOW, "GET, HEAD")
                        .body("This resource only supports GET and HEAD."),
                ));
            }
        }

        let req_path = req.match_info().path();

        let mut item = self.files.get(req_path);

        if item.is_none()
            && self.resolve_defaults
            && (req_path.is_empty() || req_path.ends_with("/"))
        {
            let index_req_path = req_path.to_string() + INDEX_HTML;
            item = self.files.get(index_req_path.as_str());
        }

        let (req, response) = if item.is_some() {
            let (req, _) = req.into_parts();
            let response = respond_to(&req, item);
            (req, response)
        } else {
            let real_path = match get_pathbuf(req_path) {
                Ok(item) => item,
                Err(e) => return ok(req.error_response(e)),
            };

            let (req, _) = req.into_parts();

            let mut item = self.files.get(real_path.as_str());

            if item.is_none() && self.not_found_resolves_to.is_some() {
                let not_found_path = self.not_found_resolves_to.as_ref().unwrap();
                item = self.files.get(not_found_path.as_str());
            }

            let response = respond_to(&req, item);
            (req, response)
        };

        ok(ServiceResponse::new(req, response))
    }
}

fn respond_to(req: &HttpRequest, item: Option<&Resource>) -> HttpResponse {
    if let Some(file) = item {
        let etag = Some(header::EntityTag::strong(format!(
            "{:x}:{:x}",
            file.data.len(),
            file.modified
        )));

        let precondition_failed = !any_match(etag.as_ref(), req);

        let not_modified = !none_match(etag.as_ref(), req);

        let mut resp = HttpResponse::build(StatusCode::OK);
        resp.set_header(header::CONTENT_TYPE, file.mime_type);

        if let Some(etag) = etag {
            resp.set(header::ETag(etag));
        }

        if precondition_failed {
            return resp.status(StatusCode::PRECONDITION_FAILED).finish();
        } else if not_modified {
            return resp.status(StatusCode::NOT_MODIFIED).finish();
        }

        resp.body(file.data)
    } else {
        HttpResponse::NotFound().body("Not found")
    }
}

/// Returns true if `req` has no `If-Match` header or one which matches `etag`.
fn any_match(etag: Option<&header::EntityTag>, req: &HttpRequest) -> bool {
    match req.get_header::<header::IfMatch>() {
        None | Some(header::IfMatch::Any) => true,
        Some(header::IfMatch::Items(ref items)) => {
            if let Some(some_etag) = etag {
                for item in items {
                    if item.strong_eq(some_etag) {
                        return true;
                    }
                }
            }
            false
        }
    }
}

/// Returns true if `req` doesn't have an `If-None-Match` header matching `req`.
fn none_match(etag: Option<&header::EntityTag>, req: &HttpRequest) -> bool {
    match req.get_header::<header::IfNoneMatch>() {
        Some(header::IfNoneMatch::Any) => false,
        Some(header::IfNoneMatch::Items(ref items)) => {
            if let Some(some_etag) = etag {
                for item in items {
                    if item.weak_eq(some_etag) {
                        return false;
                    }
                }
            }
            true
        }
        None => true,
    }
}

#[derive(Debug, PartialEq, Display, Error)]
pub enum UriSegmentError {
    /// The segment started with the wrapped invalid character.
    #[display(fmt = "The segment started with the wrapped invalid character")]
    BadStart(#[error(not(source))] char),

    /// The segment contained the wrapped invalid character.
    #[display(fmt = "The segment contained the wrapped invalid character")]
    BadChar(#[error(not(source))] char),

    /// The segment ended with the wrapped invalid character.
    #[display(fmt = "The segment ended with the wrapped invalid character")]
    BadEnd(#[error(not(source))] char),
}

#[cfg(test)]
mod tests_error_impl {
    use super::*;

    fn assert_send_and_sync<T: Send + Sync + 'static>() {}

    #[test]
    fn test_error_impl() {
        // ensure backwards compatibility when migrating away from failure
        assert_send_and_sync::<UriSegmentError>();
    }
}

/// Return `BadRequest` for `UriSegmentError`
impl ResponseError for UriSegmentError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::new(StatusCode::BAD_REQUEST)
    }
}

fn get_pathbuf(path: &str) -> Result<String, UriSegmentError> {
    let mut buf = Vec::new();
    for segment in path.split('/') {
        if segment == ".." {
            buf.pop();
        } else if segment.starts_with('.') {
            return Err(UriSegmentError::BadStart('.'));
        } else if segment.starts_with('*') {
            return Err(UriSegmentError::BadStart('*'));
        } else if segment.ends_with(':') {
            return Err(UriSegmentError::BadEnd(':'));
        } else if segment.ends_with('>') {
            return Err(UriSegmentError::BadEnd('>'));
        } else if segment.ends_with('<') {
            return Err(UriSegmentError::BadEnd('<'));
        } else if segment.is_empty() {
            continue;
        } else if cfg!(windows) && segment.contains('\\') {
            return Err(UriSegmentError::BadChar('\\'));
        } else {
            buf.push(segment)
        }
    }

    Ok(buf.join("/"))
}

fn collect_resources<P: AsRef<Path>>(
    path: P,
    filter: Option<fn(p: &Path) -> bool>,
) -> io::Result<Vec<(PathBuf, Metadata)>> {
    let mut result = vec![];

    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(ref filter) = filter {
            if !filter(path.as_ref()) {
                continue;
            }
        }

        if path.is_dir() {
            let nested = collect_resources(path, filter)?;
            result.extend(nested);
        } else {
            result.push((path, entry.metadata()?));
        }
    }

    Ok(result)
}

/// Generate resources for `resource_dir`.
///
/// ```rust
/// // Generate resources for ./tests dir with file name generated.rs
/// // stored in path defined by OUT_DIR environment variable.
/// // Function name is 'generate'
/// use actix_web_static_files::resource_dir;
///
/// resource_dir("./tests").build().unwrap();
/// ```
pub fn resource_dir<P: AsRef<Path>>(resource_dir: P) -> ResourceDir {
    ResourceDir {
        resource_dir: resource_dir.as_ref().into(),
        ..Default::default()
    }
}

#[derive(Default)]
pub struct ResourceDir {
    resource_dir: PathBuf,
    filter: Option<fn(p: &Path) -> bool>,
    generated_filename: Option<PathBuf>,
    generated_fn: Option<String>,
}

impl ResourceDir {
    pub fn build(&self) -> io::Result<()> {
        let generated_filename = self.generated_filename.clone().unwrap_or_else(|| {
            let out_dir = env::var("OUT_DIR").unwrap();

            Path::new(&out_dir).join("generated.rs")
        });
        let generated_fn = self
            .generated_fn
            .clone()
            .unwrap_or_else(|| "generate".into());

        generate_resources(
            &self.resource_dir,
            self.filter,
            &generated_filename,
            &generated_fn,
        )
    }

    pub fn with_filter(&mut self, filter: fn(p: &Path) -> bool) -> &mut Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_generated_filename<P: AsRef<Path>>(&mut self, generated_filename: P) -> &mut Self {
        self.generated_filename = Some(generated_filename.as_ref().into());
        self
    }

    pub fn with_generated_fn(&mut self, generated_fn: impl Into<String>) -> &mut Self {
        self.generated_fn = Some(generated_fn.into());
        self
    }
}

const DEFAULT_VARIABLE_NAME: &str = "r";

/// Generate resources for `project_dir` using `filter`.
/// Result saved in `generated_filename` and function named as `fn_name`.
///
/// in `build.rs`:
/// ```rust
///
/// use std::env;
/// use std::path::Path;
/// use actix_web_static_files::generate_resources;
///
/// let out_dir = env::var("OUT_DIR").unwrap();
/// let generated_filename = Path::new(&out_dir).join("generated.rs");
/// generate_resources("./tests", None, generated_filename, "generate");
/// ```
///
/// in `main.rs`:
/// ```rust
/// use actix_web::App;
///
/// include!(concat!(env!("OUT_DIR"), "/generated.rs"));
///
/// fn main() {
///     let generated_file = generate();
///
///     assert_eq!(generated_file.len(), 4);
///
///     let app = App::new()
///         .service(actix_web_static_files::ResourceFiles::new(
///            "/static",
///            generated_file,
///        ));
/// }
/// ```
pub fn generate_resources<P: AsRef<Path>, G: AsRef<Path>>(
    project_dir: P,
    filter: Option<fn(p: &Path) -> bool>,
    generated_filename: G,
    fn_name: &str,
) -> io::Result<()> {
    let resources = collect_resources(&project_dir, filter)?;

    let mut f = File::create(&generated_filename).unwrap();

    generate_function_header(&mut f, fn_name)?;
    generate_uses(&mut f)?;

    generate_variable_header(&mut f, DEFAULT_VARIABLE_NAME)?;
    generate_resource_inserts(&mut f, &project_dir, DEFAULT_VARIABLE_NAME, resources)?;
    generate_variable_return(&mut f, DEFAULT_VARIABLE_NAME)?;

    generate_function_end(&mut f)?;

    Ok(())
}

pub fn generate_resources_mapping<P: AsRef<Path>, G: AsRef<Path>>(
    project_dir: P,
    filter: Option<fn(p: &Path) -> bool>,
    generated_filename: G,
) -> io::Result<()> {
    let resources = collect_resources(&project_dir, filter)?;

    let mut f = File::create(&generated_filename).unwrap();
    writeln!(f, "{{")?;

    generate_uses(&mut f)?;

    generate_variable_header(&mut f, DEFAULT_VARIABLE_NAME)?;

    generate_resource_inserts(&mut f, &project_dir, DEFAULT_VARIABLE_NAME, resources)?;

    generate_variable_return(&mut f, DEFAULT_VARIABLE_NAME)?;

    writeln!(f, "}}")?;
    Ok(())
}

fn generate_resource_inserts<P: AsRef<Path>, W: Write>(
    f: &mut W,
    project_dir: &P,
    variable_name: &str,
    resources: Vec<(PathBuf, Metadata)>,
) -> io::Result<()> {
    for (path, metadata) in resources {
        let abs_path = path.canonicalize()?;
        let key_path = path.strip_prefix(&project_dir).unwrap().to_slash().unwrap();

        let modified = if let Ok(Ok(modified)) = metadata
            .modified()
            .map(|x| x.duration_since(SystemTime::UNIX_EPOCH))
        {
            modified.as_secs()
        } else {
            0
        };
        let mime_type = mime_guess::MimeGuess::from_path(&path).first_or_octet_stream();
        writeln!(
            f,
            "{}.insert({:?},n(i!({:?}),{:?},{:?}));",
            variable_name, &key_path, &abs_path, modified, &mime_type,
        )?;
    }
    Ok(())
}

fn generate_function_header<F: Write>(f: &mut F, fn_name: &str) -> io::Result<()> {
    writeln!(
        f,
        "#[allow(clippy::unreadable_literal)] pub fn {}() -> ::std::collections::HashMap<&'static str, ::actix_web_static_files::Resource> {{",
        fn_name
    )
}

fn generate_function_end<F: Write>(f: &mut F) -> io::Result<()> {
    writeln!(f, "}}")
}

fn generate_uses<F: Write>(f: &mut F) -> io::Result<()> {
    writeln!(
        f,
        "use ::actix_web_static_files::new_resource as n;
use ::std::include_bytes as i;",
    )
}

fn generate_variable_header<F: Write>(f: &mut F, variable_name: &str) -> io::Result<()> {
    writeln!(
        f,
        "let mut {} = ::std::collections::HashMap::new();",
        variable_name
    )
}

fn generate_variable_return<F: Write>(f: &mut F, variable_name: &str) -> io::Result<()> {
    writeln!(f, "{}", variable_name)
}

#[cfg(not(windows))]
const NPM_CMD: &str = "npm";

#[cfg(windows)]
const NPM_CMD: &str = "npm.cmd";

/// Generate resources with run of `npm install` prior to collecting
/// resources in `resource_dir`.
///
/// Resources collected in `node_modules` subdirectory.
pub fn npm_resource_dir<P: AsRef<Path>>(resource_dir: P) -> io::Result<ResourceDir> {
    Ok(NpmBuild::new(resource_dir).install()?.to_resource_dir())
}

/// Executes `npm` commands before collecting resources.
///
/// Example usage:
/// Add `build.rs` with call to bundle resources:
///
/// ```rust#ignore
/// use actix_web_static_files::NpmBuild;
///
/// fn main() {
///     NpmBuild::new("./web")
///         .install().unwrap() // runs npm install
///         .run("build").unwrap() // runs npm run build
///         .target("./web/dist")
///         .to_resource_dir()
///         .build().unwrap();
/// }
/// ```
/// Include generated code in `main.rs`:
///
/// ```rust#ignore
/// use actix_web::{App, HttpServer};
/// use actix_web_static_files;
///
/// use std::collections::HashMap;
///
/// include!(concat!(env!("OUT_DIR"), "/generated.rs"));
///
/// #[actix_rt::main]
/// async fn main() -> std::io::Result<()> {
///     HttpServer::new(move || {
///         let generated = generate();
///         App::new().service(actix_web_static_files::ResourceFiles::new(
///             "/", generated,
///         ))
///     })
///     .bind("127.0.0.1:8080")?
///     .run()
///     .await
/// }
/// ```
#[derive(Default, Debug)]
pub struct NpmBuild {
    package_json_dir: PathBuf,
    executable: String,
    target_dir: Option<PathBuf>,
}

impl NpmBuild {
    pub fn new<P: AsRef<Path>>(package_json_dir: P) -> Self {
        Self {
            package_json_dir: package_json_dir.as_ref().into(),
            executable: String::from(NPM_CMD),
            ..Default::default()
        }
    }

    /// Allow the user to set their own npm-like executable (like yarn, for instance)
    pub fn executable(self, executable: &str) -> Self {
        let executable = String::from(executable);
        Self { executable, ..self }
    }

    /// Generates change detection instructions.
    ///
    /// It includes `package.json` directory, ignores by default `node_modules`, `package.json` and `package-lock.json` and target directory.
    /// Additionally it adds `build.rs`.
    /// Each time `npm` changes timestamps on these files, so if we do not ignore them - it runs `npm` each time.
    /// It is recommended to put your dist files one level deeper. For example, if you have `web` with `package.json`
    /// and `dist` just below that, you better generate you index.html somewhere in `web\dist\sub_path\index.html`.
    /// Reason is the same, `npm` touches `dist` each time and it touches the parent directory which in its turn triggers the build each time.
    /// For complete example see: [Angular Router Sample](https://github.com/kilork/actix-web-static-files-example-angular-router).
    /// If default behavior does not work for you, you can use [change-detection](https://crates.io/crates/change-detection) directly.
    #[cfg(feature = "change-detection")]
    pub fn change_detection(self) -> Self {
        use ::change_detection::{
            path_matchers::{equal, PathMatcherExt},
            ChangeDetection,
        };

        let default_exclude_filter = equal(self.package_json_dir.join("node_modules"))
            .or(equal(self.package_json_dir.join("package.json")))
            .or(equal(self.package_json_dir.join("package-lock.json")));

        {
            let change_detection = if self.target_dir.is_none() {
                ChangeDetection::exclude(default_exclude_filter)
            } else {
                let mut target_dir = self.target_dir.clone().unwrap();

                if let Some(target_dir_parent) = target_dir.parent() {
                    if target_dir_parent.starts_with(&self.package_json_dir) {
                        while target_dir.parent() != Some(&self.package_json_dir) {
                            target_dir = target_dir.parent().unwrap().into();
                        }
                    }
                }

                let exclude_filter = default_exclude_filter.or(equal(target_dir));
                ChangeDetection::exclude(exclude_filter)
            };

            change_detection
                .path(&self.package_json_dir)
                .path("build.rs")
                .generate();
        }
        self
    }

    /// Executes `npm install`.
    pub fn install(self) -> io::Result<Self> {
        if let Err(e) = self
            .command()
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .arg("install")
            .current_dir(&self.package_json_dir)
            .status()
        {
            eprintln!("Cannot execute {} install: {:?}", &self.executable, e);
            return Err(e);
        }

        Ok(self)
    }

    /// Executes `npm run CMD`.
    pub fn run(self, cmd: &str) -> io::Result<Self> {
        if let Err(e) = self
            .command()
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .arg("run")
            .arg(cmd)
            .current_dir(&self.package_json_dir)
            .status()
        {
            eprintln!("Cannot execute {} run {}: {:?}", &self.executable, cmd, e);
            return Err(e);
        }

        Ok(self)
    }

    /// Sets target (default is node_modules).
    pub fn target<P: AsRef<Path>>(mut self, target_dir: P) -> Self {
        self.target_dir = Some(target_dir.as_ref().into());
        self
    }

    /// Converts to `ResourceDir`.
    pub fn to_resource_dir(self) -> ResourceDir {
        self.into()
    }

    #[cfg(not(windows))]
    fn command(&self) -> Command {
        Command::new(&self.executable)
    }

    #[cfg(windows)]
    fn command(&self) -> Command {
        let mut cmd = Command::new("cmd");

        cmd.arg("/c").arg(&self.executable);

        cmd
    }
}

impl From<NpmBuild> for ResourceDir {
    fn from(mut npm_build: NpmBuild) -> Self {
        Self {
            resource_dir: npm_build
                .target_dir
                .take()
                .unwrap_or_else(|| npm_build.package_json_dir.join("node_modules")),
            ..Default::default()
        }
    }
}

mod sets {}
