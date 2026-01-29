//! HTTP Router for Convex HTTP Actions
//!
//! This module provides HTTP routing capabilities for defining custom HTTP endpoints
//! in Convex applications. It supports both exact path matching and path prefix matching.
//!
//! # Example
//!
//! ```ignore
//! use convex_sdk::http::{HttpRouter, http_action};
//! use convex_sdk::{Database, json, Result};
//!
//! #[http_action]
//! pub async fn handle_webhook(ctx: HttpContext, request: Request) -> Result<Response> {
//!     let body = request.json::<serde_json::Value>().await?;
//!     // Process webhook...
//!     Ok(Response::new(200, "OK"))
//! }
//!
//! #[http_action]
//! pub async fn api_handler(ctx: HttpContext, request: Request) -> Result<Response> {
//!     // Handle API requests
//!     Ok(Response::json(200, json!({"status": "ok"})))
//! }
//!
//! // In your convex/http.rs:
//! pub fn router() -> HttpRouter {
//!     let mut router = HttpRouter::new();
//!
//!     // Exact path matching
//!     router.route(RouteSpec {
//!         path: "/webhook",
//!         method: Method::POST,
//!         handler: handle_webhook,
//!     });
//!
//!     // Path prefix matching
//!     router.route(RouteSpec {
//!         path_prefix: "/api/",
//!         method: Method::GET,
//!         handler: api_handler,
//!     });
//!
//!     router
//! }
//! ```

use crate::types::{ConvexError, Result};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// HTTP methods supported by Convex HTTP actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Method {
    /// GET request
    GET,
    /// POST request
    POST,
    /// PUT request
    PUT,
    /// DELETE request
    DELETE,
    /// PATCH request
    PATCH,
    /// OPTIONS request
    OPTIONS,
    /// HEAD request (handled as GET with body stripped)
    HEAD,
}

impl Method {
    /// Parse an HTTP method from a string
    pub fn from_str(method: &str) -> Result<Self> {
        match method.to_uppercase().as_str() {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "PATCH" => Ok(Method::PATCH),
            "OPTIONS" => Ok(Method::OPTIONS),
            "HEAD" => Ok(Method::HEAD),
            _ => Err(ConvexError::InvalidArgument(
                format!("Unsupported HTTP method: {}", method)
            )),
        }
    }

    /// Convert the method to a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
            Method::PATCH => "PATCH",
            Method::OPTIONS => "OPTIONS",
            Method::HEAD => "HEAD",
        }
    }
}

/// An HTTP request received by a Convex HTTP action
#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    /// The HTTP method
    method: Method,
    /// The request URL
    url: String,
    /// HTTP headers as key-value pairs
    headers: Vec<(String, String)>,
    /// Request body as raw bytes
    #[serde(with = "serde_bytes")]
    body: Vec<u8>,
}

impl Request {
    /// Create a new HTTP request (primarily for testing)
    pub fn new(method: Method, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Get the HTTP method
    pub fn method(&self) -> Method {
        self.method
    }

    /// Get the request URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the request path from the URL
    pub fn path(&self) -> String {
        // Simple path extraction - in production, use a proper URL parser
        self.url.split('?').next().unwrap_or(&self.url).to_string()
    }

    /// Get all headers
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// Get a specific header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Get the request body as raw bytes
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Parse the request body as JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_slice(&self.body)
            .map_err(|e| ConvexError::InvalidArgument(format!("Invalid JSON: {}", e)))
    }

    /// Get the request body as a string
    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.body.clone())
            .map_err(|e| ConvexError::InvalidArgument(format!("Invalid UTF-8: {}", e)))
    }

    /// Set the request body (for building requests)
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Add a header (for building requests)
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }
}

/// An HTTP response to be returned from a Convex HTTP action
#[derive(Debug, Clone, Serialize)]
pub struct Response {
    /// HTTP status code
    status: u16,
    /// HTTP headers as key-value pairs
    headers: Vec<(String, String)>,
    /// Response body as raw bytes
    #[serde(with = "serde_bytes")]
    body: Vec<u8>,
}

impl Response {
    /// Create a new HTTP response
    ///
    /// # Arguments
    ///
    /// * `status` - The HTTP status code
    /// * `body` - The response body as bytes
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    /// Create a successful (200 OK) response
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self::new(200, body)
    }

    /// Create a JSON response
    ///
    /// # Arguments
    ///
    /// * `status` - The HTTP status code
    /// * `data` - The data to serialize as JSON
    pub fn json<T: Serialize>(status: u16, data: &T) -> Result<Self> {
        let body = serde_json::to_vec(data)
            .map_err(|e| ConvexError::Serialization(e))?;

        let mut response = Self::new(status, body);
        response.headers.push(("Content-Type".to_string(), "application/json".to_string()));
        Ok(response)
    }

    /// Create a redirect response
    ///
    /// # Arguments
    ///
    /// * `status` - The HTTP status code (usually 302 or 301)
    /// * `location` - The URL to redirect to
    pub fn redirect(status: u16, location: impl Into<String>) -> Self {
        let mut response = Self::new(status, Vec::new());
        response.headers.push(("Location".to_string(), location.into()));
        response
    }

    /// Create a 404 Not Found response
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(404, message.into().into_bytes())
    }

    /// Create a 400 Bad Request response
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400, message.into().into_bytes())
    }

    /// Create a 401 Unauthorized response
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(401, message.into().into_bytes())
    }

    /// Create a 403 Forbidden response
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(403, message.into().into_bytes())
    }

    /// Create a 500 Internal Server Error response
    pub fn server_error(message: impl Into<String>) -> Self {
        Self::new(500, message.into().into_bytes())
    }

    /// Get the HTTP status code
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Get the response headers
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// Get the response body
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Add a header to the response
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Set the Content-Type header
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.headers.push(("Content-Type".to_string(), content_type.into()));
        self
    }
}

/// HTTP handler function type
pub type HttpHandler = fn(Request) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = Result<Response>>>>;

/// A route specification for the HTTP router
#[derive(Debug, Clone)]
pub enum RouteSpec {
    /// Exact path match
    Exact {
        /// The exact path to match (e.g., "/webhook")
        path: String,
        /// The HTTP method
        method: Method,
        /// The handler function
        handler: HttpHandler,
    },
    /// Path prefix match
    Prefix {
        /// The path prefix to match (e.g., "/api/")
        path_prefix: String,
        /// The HTTP method
        method: Method,
        /// The handler function
        handler: HttpHandler,
    },
}

impl RouteSpec {
    /// Create an exact path route
    pub fn exact(path: impl Into<String>, method: Method, handler: HttpHandler) -> Self {
        Self::Exact {
            path: path.into(),
            method,
            handler,
        }
    }

    /// Create a prefix path route
    pub fn prefix(path_prefix: impl Into<String>, method: Method, handler: HttpHandler) -> Self {
        Self::Prefix {
            path_prefix: path_prefix.into(),
            method,
            handler,
        }
    }

    /// Get the method for this route
    pub fn method(&self) -> Method {
        match self {
            RouteSpec::Exact { method, .. } => *method,
            RouteSpec::Prefix { method, .. } => *method,
        }
    }
}

/// HTTP Router for Convex HTTP Actions
///
/// Routes incoming HTTP requests to the appropriate handler based on path and method.
/// Supports both exact path matching and path prefix matching.
#[derive(Debug, Default)]
pub struct HttpRouter {
    /// Exact path routes: path -> (method -> handler)
    exact_routes: alloc::collections::BTreeMap<String, alloc::collections::BTreeMap<Method, HttpHandler>>,
    /// Prefix routes: method -> (path_prefix -> handler)
    prefix_routes: alloc::collections::BTreeMap<Method, alloc::collections::BTreeMap<String, HttpHandler>>,
    /// Marker for router identification
    pub is_router: bool,
}

impl HttpRouter {
    /// Create a new HTTP router
    pub fn new() -> Self {
        Self {
            exact_routes: alloc::collections::BTreeMap::new(),
            prefix_routes: alloc::collections::BTreeMap::new(),
            is_router: true,
        }
    }

    /// Create a new HTTP router (convenience function matching TypeScript API)
    pub fn http_router() -> Self {
        Self::new()
    }

    /// Add a route to the router
    ///
    /// # Arguments
    ///
    /// * `spec` - The route specification
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The path doesn't start with "/"
    /// - The path is reserved (starts with "/.files/")
    /// - The route conflicts with an existing route
    pub fn route(&mut self, spec: RouteSpec) -> Result<&mut Self> {
        match spec {
            RouteSpec::Exact { path, method, handler } => {
                self.add_exact_route(path, method, handler)?;
            }
            RouteSpec::Prefix { path_prefix, method, handler } => {
                self.add_prefix_route(path_prefix, method, handler)?;
            }
        }
        Ok(self)
    }

    /// Add an exact path route
    fn add_exact_route(
        &mut self,
        path: String,
        method: Method,
        handler: HttpHandler,
    ) -> Result<()> {
        // Validate path
        if !path.starts_with('/') {
            return Err(ConvexError::InvalidArgument(
                format!("Path '{}' must start with /", path)
            ));
        }

        if path.starts_with("/.files/") || path == "/.files" {
            return Err(ConvexError::InvalidArgument(
                format!("Path '{}' is reserved", path)
            ));
        }

        // Add route
        let methods = self.exact_routes.entry(path.clone()).or_default();

        if methods.contains_key(&method) {
            return Err(ConvexError::InvalidArgument(
                format!("Route for {} {} already exists", method.as_str(), path)
            ));
        }

        methods.insert(method, handler);
        Ok(())
    }

    /// Add a prefix path route
    fn add_prefix_route(
        &mut self,
        path_prefix: String,
        method: Method,
        handler: HttpHandler,
    ) -> Result<()> {
        // Validate path prefix
        if !path_prefix.starts_with('/') {
            return Err(ConvexError::InvalidArgument(
                format!("Path prefix '{}' must start with /", path_prefix)
            ));
        }

        if !path_prefix.ends_with('/') {
            return Err(ConvexError::InvalidArgument(
                format!("Path prefix '{}' must end with /", path_prefix)
            ));
        }

        if path_prefix.starts_with("/.files/") {
            return Err(ConvexError::InvalidArgument(
                format!("Path prefix '{}' is reserved", path_prefix)
            ));
        }

        // Add route
        let prefixes = self.prefix_routes.entry(method).or_default();

        if prefixes.contains_key(&path_prefix) {
            return Err(ConvexError::InvalidArgument(
                format!("Prefix route for {} {} already exists", method.as_str(), path_prefix)
            ));
        }

        prefixes.insert(path_prefix, handler);
        Ok(())
    }

    /// Look up a route for the given path and method
    ///
    /// # Arguments
    ///
    /// * `path` - The request path
    /// * `method` - The HTTP method
    ///
    /// # Returns
    ///
    /// `Some((handler, matched_path))` if a route is found, `None` otherwise.
    /// For exact matches, `matched_path` is the exact path.
    /// For prefix matches, `matched_path` is the prefix with "*" appended.
    pub fn lookup(&self, path: &str, method: Method) -> Option<(HttpHandler, String)> {
        // Normalize HEAD to GET for routing
        let lookup_method = if method == Method::HEAD {
            Method::GET
        } else {
            method
        };

        // Try exact match first
        if let Some(methods) = self.exact_routes.get(path) {
            if let Some(&handler) = methods.get(&lookup_method) {
                return Some((handler, path.to_string()));
            }
        }

        // Try prefix matches (longest prefix wins)
        if let Some(prefixes) = self.prefix_routes.get(&lookup_method) {
            let mut matches: Vec<_> = prefixes
                .iter()
                .filter(|(prefix, _)| path.starts_with(*prefix))
                .collect();

            // Sort by prefix length descending (longest match wins)
            matches.sort_by_key(|(prefix, _)| -(prefix.len() as isize));

            if let Some((prefix, &handler)) = matches.first() {
                return Some((handler, format!("{}*", prefix)));
            }
        }

        None
    }

    /// Get all registered routes
    ///
    /// # Returns
    ///
    /// A vector of (path, method, handler) tuples, sorted by path and method.
    /// For prefix routes, the path ends with "*".
    pub fn get_routes(&self) -> Vec<(String, Method, HttpHandler)> {
        let mut routes = Vec::new();

        // Add exact routes
        for (path, methods) in &self.exact_routes {
            for (method, &handler) in methods {
                routes.push((path.clone(), *method, handler));
            }
        }

        // Add prefix routes
        for (method, prefixes) in &self.prefix_routes {
            for (prefix, &handler) in prefixes {
                routes.push((format!("{}*", prefix), *method, handler));
            }
        }

        routes
    }

    /// Handle an incoming request
    ///
    /// # Arguments
    ///
    /// * `request` - The incoming HTTP request
    ///
    /// # Returns
    ///
    /// The HTTP response from the matched handler, or a 404 response if no route matches.
    pub async fn handle(&self, request: Request) -> Response {
        let path = request.path();
        let method = request.method();

        match self.lookup(&path, method) {
            Some((handler, _)) => {
                match handler(request).await {
                    Ok(response) => response,
                    Err(e) => Response::server_error(format!("Handler error: {}", e)),
                }
            }
            None => Response::not_found(format!("No route for {} {}", method.as_str(), path)),
        }
    }

    /// Export the router configuration as JSON
    ///
    /// This is used internally by the Convex framework.
    pub fn export(&self) -> String {
        let routes: Vec<_> = self.get_routes()
            .into_iter()
            .map(|(path, method, _)| {
                serde_json::json!({
                    "path": path,
                    "method": method.as_str(),
                })
            })
            .collect();

        serde_json::to_string(&routes).unwrap_or_default()
    }
}

/// Context available to HTTP actions
///
/// This provides access to Convex functionality within HTTP action handlers.
#[derive(Debug)]
pub struct HttpContext {
    // In a full implementation, this would include:
    // - run_query: Function to run queries
    // - run_mutation: Function to run mutations
    // - run_action: Function to run actions
}

impl HttpContext {
    /// Create a new HTTP context
    pub fn new() -> Self {
        Self {}
    }

    /// Run a query from within an HTTP action
    ///
    /// Note: This is a placeholder. In a full implementation, this would
    /// call back into the Convex runtime to execute a query.
    pub async fn run_query<T>(&self, _query_ref: &str, _args: serde_json::Value) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // Placeholder - would call into host
        Err(ConvexError::Unknown("run_query not yet implemented".into()))
    }

    /// Run a mutation from within an HTTP action
    ///
    /// Note: This is a placeholder. In a full implementation, this would
    /// call back into the Convex runtime to execute a mutation.
    pub async fn run_mutation<T>(&self, _mutation_ref: &str, _args: serde_json::Value) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // Placeholder - would call into host
        Err(ConvexError::Unknown("run_mutation not yet implemented".into()))
    }
}

impl Default for HttpContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_handler(_req: Request) -> core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = Result<Response>>>> {
        Box::pin(async { Ok(Response::ok("")) })
    }

    #[test]
    fn test_method_parsing() {
        assert_eq!(Method::from_str("GET").unwrap(), Method::GET);
        assert_eq!(Method::from_str("post").unwrap(), Method::POST);
        assert_eq!(Method::from_str("DELETE").unwrap(), Method::DELETE);
        assert!(Method::from_str("INVALID").is_err());
    }

    #[test]
    fn test_router_exact_match() {
        let mut router = HttpRouter::new();

        router.route(RouteSpec::exact("/webhook", Method::POST, dummy_handler)).unwrap();

        let (handler, matched) = router.lookup("/webhook", Method::POST).unwrap();
        assert_eq!(matched, "/webhook");

        // Different method should not match
        assert!(router.lookup("/webhook", Method::GET).is_none());

        // Different path should not match
        assert!(router.lookup("/other", Method::POST).is_none());
    }

    #[test]
    fn test_router_prefix_match() {
        let mut router = HttpRouter::new();

        router.route(RouteSpec::prefix("/api/", Method::GET, dummy_handler)).unwrap();

        // Should match paths starting with /api/
        let (handler, matched) = router.lookup("/api/users", Method::GET).unwrap();
        assert_eq!(matched, "/api/*");

        let (handler, matched) = router.lookup("/api/v1/posts", Method::GET).unwrap();
        assert_eq!(matched, "/api/*");

        // Should not match exact /api (without trailing slash)
        assert!(router.lookup("/api", Method::GET).is_none());

        // Should not match different method
        assert!(router.lookup("/api/users", Method::POST).is_none());
    }

    #[test]
    fn test_router_longest_prefix_wins() {
        let mut router = HttpRouter::new();

        router.route(RouteSpec::prefix("/api/", Method::GET, dummy_handler)).unwrap();
        router.route(RouteSpec::prefix("/api/v2/", Method::GET, dummy_handler)).unwrap();

        let (_, matched) = router.lookup("/api/v2/users", Method::GET).unwrap();
        assert_eq!(matched, "/api/v2/*");

        let (_, matched) = router.lookup("/api/v1/users", Method::GET).unwrap();
        assert_eq!(matched, "/api/*");
    }

    #[test]
    fn test_router_exact_over_prefix() {
        let mut router = HttpRouter::new();

        router.route(RouteSpec::prefix("/api/", Method::GET, dummy_handler)).unwrap();
        router.route(RouteSpec::exact("/api/special", Method::GET, dummy_handler)).unwrap();

        // Exact match should win
        let (_, matched) = router.lookup("/api/special", Method::GET).unwrap();
        assert_eq!(matched, "/api/special");
    }

    #[test]
    fn test_router_validation() {
        let mut router = HttpRouter::new();

        // Path must start with /
        assert!(router.route(RouteSpec::exact("webhook", Method::POST, dummy_handler)).is_err());

        // Reserved path
        assert!(router.route(RouteSpec::exact("/.files/test", Method::GET, dummy_handler)).is_err());

        // Prefix must end with /
        assert!(router.route(RouteSpec::prefix("/api", Method::GET, dummy_handler)).is_err());
    }

    #[test]
    fn test_request_helpers() {
        let request = Request::new(Method::POST, "https://example.com/webhook?foo=bar")
            .with_header("Content-Type", "application/json")
            .with_body(b"{\"test\": true}".to_vec());

        assert_eq!(request.method(), Method::POST);
        assert_eq!(request.header("Content-Type"), Some("application/json"));
        assert_eq!(request.header("content-type"), Some("application/json")); // Case insensitive
        assert_eq!(request.text().unwrap(), "{\"test\": true}");
    }

    #[test]
    fn test_response_helpers() {
        let response = Response::json(200, &serde_json::json!({"status": "ok"})).unwrap();
        assert_eq!(response.status(), 200);
        let content_type = response.headers().iter().find(|(k, _)| k == "Content-Type").map(|(_, v)| v.as_str());
        assert_eq!(content_type, Some("application/json"));

        let redirect = Response::redirect(302, "https://example.com");
        assert_eq!(redirect.status(), 302);
        let location = redirect.headers().iter().find(|(k, _)| k == "Location").map(|(_, v)| v.as_str());
        assert_eq!(location, Some("https://example.com"));
    }
}
