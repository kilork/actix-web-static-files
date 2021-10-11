use actix_service::{Service, ServiceFactory};
use actix_web::{
    dev::{AppService, HttpServiceFactory, ResourceDef, ServiceRequest, ServiceResponse},
    error::Error,
    http::{header, Method, StatusCode},
    HttpMessage, HttpRequest, HttpResponse, ResponseError,
};
use derive_more::{Display, Error};
use futures::future::{ok, FutureExt, LocalBoxFuture, Ready};
use static_files::Resource;
use std::{
    collections::HashMap,
    ops::Deref,
    rc::Rc,
    task::{Context, Poll},
};

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
///     let files: HashMap<&'static str, static_files::Resource> = HashMap::new();
///     let app = App::new()
///         .service(actix_web_static_files::ResourceFiles::new("/", files));
/// // or subpath with additional option to not resolve index.html
///     let files: HashMap<&'static str, static_files::Resource> = HashMap::new();
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

impl ServiceFactory<ServiceRequest> for ResourceFiles {
    type Config = ();
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

impl<'a> Service<ServiceRequest> for ResourceFilesService {
    type Response = ServiceResponse;
    type Error = Error;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        match *req.method() {
            Method::HEAD | Method::GET => (),
            _ => {
                return ok(ServiceResponse::new(
                    req.into_parts().0,
                    HttpResponse::MethodNotAllowed()
                        .append_header((header::CONTENT_TYPE, "text/plain"))
                        .append_header((header::ALLOW, "GET, HEAD"))
                        .body("This resource only supports GET and HEAD."),
                ));
            }
        }

        let req_path = req.match_info().path();

        let mut item = self.files.get(req_path);

        if item.is_none()
            && self.resolve_defaults
            && (req_path.is_empty() || req_path.ends_with('/'))
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
        resp.insert_header((header::CONTENT_TYPE, file.mime_type));

        if let Some(etag) = etag {
            resp.insert_header(header::ETag(etag));
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
