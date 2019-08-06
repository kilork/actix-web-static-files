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
/// use std::path::PathBuf;
///
/// use actix_web::App;
///
/// fn main() {
///     let files: HashMap<PathBuf, actix_web_static_files::Resource> = HashMap::new();
///     let app = App::new()
///         .service(actix_web_static_files::ResourceFiles::new(".", files));
/// }
/// ```
pub struct ResourceFiles {
    inner: Rc<ResourceFilesInner>,
}

pub struct ResourceFilesInner {
    path: String,
    files: HashMap<PathBuf, Resource>,
}

impl ResourceFiles {
    pub fn new(path: &str, files: HashMap<PathBuf, Resource>) -> Self {
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

    Ok(if let Some(file) = service.files.get(path) {
        HttpResponse::Ok().body(file.data)
    } else {
        HttpResponse::NotFound().body("Not found")
    })
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

/// Generate resources for `project_dir` using `filter`.
/// Result saved in `generated_filename` and function named as `fn_name`.
///
/// ```rust
/// // should be in `build.rs` file.
/// use std::env;
/// use std::path::Path;
/// use actix_web_static_files::generate_resources;
///
/// let out_dir = env::var("OUT_DIR").unwrap();
/// let generated_filename = Path::new(&out_dir).join("generated_file.rs");
/// generate_resources("./tests", None, generated_filename, "generate");
///
/// // in `main.rs`
/// // we use in fact `include!(concat!(env!("OUT_DIR"), "/generated_file.rs"));`
/// // here is just sample to check file content is generated.
/// let generated_file = std::fs::read_to_string(concat!(env!("OUT_DIR"), "/generated_file.rs")).unwrap();
///
/// assert!(generated_file.contains("pub fn generate() -> HashMap<std::path::PathBuf, actix_web_static_files::Resource> {\nlet mut result"));
/// assert!(generated_file.contains("file1.txt"));
/// assert!(generated_file.contains("file2.txt"));
/// assert!(generated_file.contains("file3.info"));
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
        "pub fn {}() -> HashMap<std::path::PathBuf, actix_web_static_files::Resource> {{",
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
            "result.insert({:?}.into(), actix_web_static_files::Resource {{ data, modified }});",
            &path,
        )?;
        writeln!(f, "}}")?;
    }

    writeln!(f, "result")?;
    writeln!(f, "}}")?;

    Ok(())
}
