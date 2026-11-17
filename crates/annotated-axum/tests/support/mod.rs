use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::http::{Request, header};
use axum::response::Response;
use http_body::Frame;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// What `app` answers to a `POST /` with `body`, sent as `content_type`.
pub async fn post(app: Router, content_type: &'static str, body: &'static str) -> Response {
    let request = Request::post("/")
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(body))
        .expect("a request with a body");

    app.oneshot(request).await.expect("a router always answers")
}

/// What `app` answers to a `POST /` with `body`, sent without a `Content-Type`.
pub async fn post_without_a_content_type(app: Router, body: &'static str) -> Response {
    let request = Request::post("/")
        .body(Body::from(body))
        .expect("a request with a body");

    app.oneshot(request).await.expect("a router always answers")
}

/// What `app` answers to a `POST /` of JSON whose body fails while it is read, as when the connection breaks.
pub async fn post_a_body_that_fails(app: Router) -> Response {
    let request = Request::post("/")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::new(FailingBody))
        .expect("a request with a body");

    app.oneshot(request).await.expect("a router always answers")
}

pub fn content_type_of(response: &Response) -> &str {
    response.headers()[header::CONTENT_TYPE]
        .to_str()
        .expect("a content type in ASCII")
}

pub async fn text_body_of(response: Response) -> String {
    let body = response.into_body().collect().await.expect("a whole body").to_bytes();
    String::from_utf8(body.to_vec()).expect("a body in UTF-8")
}

pub async fn json_body_of(response: Response) -> Value {
    let body = response.into_body().collect().await.expect("a whole body").to_bytes();
    serde_json::from_slice(&body).expect("a body in JSON")
}

struct FailingBody;

impl http_body::Body for FailingBody {
    type Data = Bytes;
    type Error = io::Error;

    fn poll_frame(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Result<Frame<Bytes>, io::Error>>> {
        Poll::Ready(Some(Err(io::Error::other("the connection broke"))))
    }
}
