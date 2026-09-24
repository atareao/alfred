use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

pub struct TestApp {
    pub router: axum::Router,
}

impl TestApp {
    pub async fn new() -> Self {
        let state = alfred::AppState::new_in_memory().await;
        let router = alfred::app_with_state(state);
        Self { router }
    }

    /// Creates a TestApp with NO seed data (clean database).
    /// Only migrations and default tools are applied.
    pub async fn new_empty() -> Self {
        let state = alfred::AppState::new_in_memory_empty().await;
        let router = alfred::app_with_state(state);
        Self { router }
    }

    pub async fn get(&self, path: &str) -> TestResponse {
        let req = Request::builder()
            .uri(path)
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        TestResponse { resp }
    }

    pub fn post(&self, path: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), Method::POST, path)
    }

    pub fn put(&self, path: &str) -> TestRequestBuilder {
        TestRequestBuilder::new(self.router.clone(), Method::PUT, path)
    }

    pub async fn delete(&self, path: &str) -> TestResponse {
        let req = Request::builder()
            .uri(path)
            .method(Method::DELETE)
            .body(Body::empty())
            .unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        TestResponse { resp }
    }
}

pub struct TestRequestBuilder {
    router: axum::Router,
    method: Method,
    path: String,
    body: Option<String>,
}

impl TestRequestBuilder {
    pub fn new(router: axum::Router, method: Method, path: &str) -> Self {
        Self {
            router,
            method,
            path: path.to_string(),
            body: None,
        }
    }

    pub fn json(mut self, value: &Value) -> Self {
        self.body = Some(value.to_string());
        self
    }

    pub async fn send(self) -> TestResponse {
        let mut builder = Request::builder().uri(&self.path).method(&self.method);
        if let Some(body) = self.body {
            builder = builder.header("content-type", "application/json");
            let req = builder.body(Body::from(body)).unwrap();
            let resp = self.router.clone().oneshot(req).await.unwrap();
            return TestResponse { resp };
        }
        let req = builder.body(Body::empty()).unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        TestResponse { resp }
    }
}

pub struct TestResponse {
    resp: axum::response::Response,
}

impl TestResponse {
    pub fn status(&self) -> StatusCode {
        self.resp.status()
    }

    pub fn headers(&self) -> &axum::http::HeaderMap {
        self.resp.headers()
    }

    pub async fn json<T: serde::de::DeserializeOwned>(self) -> T {
        let body = axum::body::to_bytes(self.resp.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }
}
