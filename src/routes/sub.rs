#![allow(unused)]
use axum::{routing::get, Router};

pub fn sub_router() -> Router {
    Router::new()
        .route("/sub", get(sub_handler))
}

async fn sub_handler() -> &'static str {
    "Subscription handler"
}