//! A server that accepts new members at `POST /members`, and responds with every issue in a body at once.

use annotated::Deserialize;
use annotated_example_axum::ValidJson;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde::Serialize;
use tokio::net::TcpListener;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

async fn add_member(ValidJson(new_member): ValidJson<NewMember>) -> (StatusCode, Json<NewMember>) {
    (StatusCode::CREATED, Json(new_member))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/members", post(add_member));
    let listener = TcpListener::bind("127.0.0.1:3000").await.expect("port 3000 is free");
    println!("Taking new members at http://127.0.0.1:3000/members");
    axum::serve(listener, app)
        .await
        .expect("the server runs until it's stopped");
}
