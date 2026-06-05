use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AdminToken(pub Arc<String>);

/// Middleware: requires `Authorization: Bearer <ADMIN_TOKEN>` on protected routes.
pub async fn require_admin(request: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let token = request.extensions().get::<AdminToken>().cloned();
    let Some(AdminToken(expected)) = token else {
        // No admin token configured → allow (dev mode)
        return Ok(next.run(request).await);
    };

    match extract_bearer(request.headers()) {
        Some(t) if t == expected.as_str() => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

fn extract_bearer(headers: &HeaderMap) -> Option<&str> {
    headers.get("authorization")?.to_str().ok()?.strip_prefix("Bearer ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_bearer_valid() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer my-secret-token"));
        assert_eq!(extract_bearer(&headers), Some("my-secret-token"));
    }

    #[test]
    fn test_extract_bearer_missing() {
        let headers = HeaderMap::new();
        assert_eq!(extract_bearer(&headers), None);
    }

    #[test]
    fn test_extract_bearer_wrong_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Basic abc123"));
        assert_eq!(extract_bearer(&headers), None);
    }

    #[test]
    fn test_extract_bearer_empty_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer "));
        assert_eq!(extract_bearer(&headers), Some(""));
    }
}
