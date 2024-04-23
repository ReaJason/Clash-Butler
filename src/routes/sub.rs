#![allow(unused)]
use axum::{Router, routing::get};

pub fn sub_router() -> Router {
    Router::new()
        .route("/sub", get(sub_handler))
}

async fn sub_handler() -> &'static str {
    "Subscription handler"
}