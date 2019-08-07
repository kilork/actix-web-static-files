use actix_http::body::SizedStream;
use actix_service::{NewService, Service};
use actix_web::{
    dev::{AppService, HttpServiceFactory, ResourceDef, ServiceRequest, ServiceResponse},
    error::{BlockingError, Error, ErrorInternalServerError},
    http::{
        header::{self, ContentDisposition, DispositionParam, DispositionType},
        ContentEncoding, Method, StatusCode,
    },
    HttpRequest, HttpResponse, ResponseError,
};
use failure::Fail;
use futures::{
    future::{ok, Either, FutureResult},
    Async, Future, Poll, Stream,
};
use mime::Mime;
use std::{
    collections::HashMap,
    env,
    fs::{self, File, Metadata},
    io::{self, Write},
    ops::Deref,
    path::{Path, PathBuf},
    rc::Rc,
    time::SystemTime,
};

/// Static files resource.
pub struct Resource {
    pub data: &'static [u8],
    pub modified: u64,
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
///     let files: HashMap<&'static str, actix_web_static_files::Resource> = HashMap::new();
///     let app = App::new()
///         .service(actix_web_static_files::ResourceFiles::new(".", files));
/// }
/// ```
pub struct ResourceFiles {
    inner: Rc<ResourceFilesInner>,
}

pub struct ResourceFilesInner {
    path: String,
    files: HashMap<&'static str, Resource>,
}

impl ResourceFiles {
    pub fn new(path: &str, files: HashMap<&'static str, Resource>) -> Self {
        let inner = ResourceFilesInner {
            path: path.into(),
            files,
        };
        Self {
            inner: Rc::new(inner),
        }
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

impl NewService for ResourceFiles {
    type Config = ();
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = Error;
    type Service = ResourceFilesService;
    type InitError = ();
    type Future = Box<dyn Future<Item = Self::Service, Error = Self::InitError>>;

    fn new_service(&self, _: &()) -> Self::Future {
        Box::new(ok(ResourceFilesService {
            inner: self.inner.clone(),
        }))
    }
}

pub struct ResourceFilesService {
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
    type Future = Either<
        FutureResult<Self::Response, Self::Error>,
        Box<dyn Future<Item = Self::Response, Error = Self::Error>>,
    >;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }
    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let real_path = match get_pathbuf(req.match_info().path()) {
            Ok(item) => item,
            Err(e) => return Either::A(ok(req.error_response(e))),
        };

        let (req, _) = req.into_parts();

        Either::A(ok(match respond_to(&req, &real_path, &self) {
            Ok(item) => ServiceResponse::new(req.clone(), item),
            Err(e) => ServiceResponse::from_err(e, req),
        }))
    }
}

fn respond_to(
    req: &HttpRequest,
    path: &Path,
    service: &ResourceFilesService,
) -> Result<HttpResponse, Error> {
    match *req.method() {
        Method::HEAD | Method::GET => (),
        _ => {
            return Ok(HttpResponse::MethodNotAllowed()
                .header(header::CONTENT_TYPE, "text/plain")
                .header(header::ALLOW, "GET, HEAD")
                .body("This resource only supports GET and HEAD."));
        }
    }

    Ok(
        if let Some(file) = path.to_str().and_then(|x| service.files.get(x)) {
            HttpResponse::Ok().body(file.data)
        } else {
            HttpResponse::NotFound().body("Not found")
        },
    )
}

#[derive(Fail, Debug, PartialEq)]
pub enum UriSegmentError {
    /// The segment started with the wrapped invalid character.
    #[fail(display = "The segment started with the wrapped invalid character")]
    BadStart(char),
    /// The segment contained the wrapped invalid character.
    #[fail(display = "The segment contained the wrapped invalid character")]
    BadChar(char),
    /// The segment ended with the wrapped invalid character.
    #[fail(display = "The segment ended with the wrapped invalid character")]
    BadEnd(char),
}

/// Return `BadRequest` for `UriSegmentError`
impl ResponseError for UriSegmentError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::new(StatusCode::BAD_REQUEST)
    }
}

fn get_pathbuf(path: &str) -> Result<PathBuf, UriSegmentError> {
    let mut buf = PathBuf::new();
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

    Ok(buf)
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
            result.push((path.into(), entry.metadata()?));
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
///
/// Can be extended with advanced configuration if required.
///
pub fn resource_dir<P: AsRef<Path>>(resource_dir: P) -> ResourceDir {
    ResourceDir {
        resource_dir: resource_dir.as_ref().into(),
        ..Default::default()
    }
}

impl ResourceDir {
    pub fn build(self) -> io::Result<()> {
        let generated_filename = self.generated_filename.unwrap_or_else(|| {
            let out_dir = env::var("OUT_DIR").unwrap();

            Path::new(&out_dir).join("generated.rs")
        });
        let generated_fn = self.generated_fn.unwrap_or_else(|| "generate".into());

        generate_resources(
            self.resource_dir,
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

#[derive(Default)]
pub struct ResourceDir {
    resource_dir: PathBuf,
    filter: Option<fn(p: &Path) -> bool>,
    generated_filename: Option<PathBuf>,
    generated_fn: Option<String>,
}

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
/// use std::collections::HashMap;
/// use actix_web::App;
///
/// include!(concat!(env!("OUT_DIR"), "/generated.rs"));
/// // here is just sample to check file content is generated.
/// fn main() {
///     let generated_file = generate();
///
///     assert_eq!(generated_file.len(), 3);
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

    writeln!(
        f,
        "pub fn {}() -> HashMap<&'static str, actix_web_static_files::Resource> {{",
        fn_name
    )?;
    writeln!(f, "let mut result = HashMap::new();")?;

    for (path, metadata) in resources {
        let abs_path = path.canonicalize()?;
        let path = path.strip_prefix(&project_dir).unwrap();

        writeln!(f, "{{")?;
        writeln!(f, "let data = include_bytes!({:?});", &abs_path,)?;

        if let Ok(Ok(modified)) = metadata
            .modified()
            .map(|x| x.duration_since(SystemTime::UNIX_EPOCH))
        {
            writeln!(f, "let modified = {:?};", modified.as_secs())?;
        } else {
            writeln!(f, "let modified = 0;")?;
        }

        writeln!(
            f,
            "result.insert({:?}, actix_web_static_files::Resource {{ data, modified }});",
            &path,
        )?;
        writeln!(f, "}}")?;
    }

    writeln!(f, "result")?;
    writeln!(f, "}}")?;

    Ok(())
}
