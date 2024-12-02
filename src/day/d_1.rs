use axum::{routing::get, Router};

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(task1::hello_world))
        .route("/-1/seek", get(task2::seek))
}

mod task1 {
    pub async fn hello_world() -> &'static str {
        "Hello, bird!"
    }
}

mod task2 {
    use axum::{
        http::{header, StatusCode},
        response::IntoResponse,
    };

    pub async fn seek() -> impl IntoResponse {
        (
            StatusCode::FOUND,
            [(
                header::LOCATION,
                "https://www.youtube.com/watch?v=9Gc4QTqslN4",
            )],
        )
    }
}
