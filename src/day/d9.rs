use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use leaky_bucket::RateLimiter;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Debug, Clone)]
pub struct Milk {
    limiter: Arc<Mutex<RateLimiter>>,
}

impl Milk {
    fn new() -> Self {
        Self {
            limiter: Arc::new(Mutex::new(new_limiter())),
        }
    }
}

fn new_limiter() -> RateLimiter {
    RateLimiter::builder()
        .initial(5)
        .max(5)
        .refill(1)
        .interval(Duration::from_secs(1))
        .build()
}

pub fn get_routes() -> Router {
    let state = Milk::new();

    Router::new()
        .route("/9/milk", post(milk))
        .route("/9/refill", post(refill))
        .with_state(state)
}

#[skip_serializing_none]
#[derive(Debug, Default, Deserialize, Serialize)]
struct MilkUnits {
    gallons: Option<f32>,
    liters: Option<f32>,
    litres: Option<f32>,
    pints: Option<f32>,
    #[serde(flatten, skip_serializing)]
    extra: HashMap<String, serde_json::Value>,
}

pub async fn milk(
    headers: HeaderMap,
    State(state): State<Milk>,
    body: String,
) -> Result<impl IntoResponse, impl IntoResponse> {
    if !state.limiter.lock().unwrap().try_acquire(1) {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            "No milk available\n".to_string(),
        ));
    }

    if let Some(b"application/json") = headers.get("Content-Type").map(|header| header.as_bytes()) {
        let units: MilkUnits =
            serde_json::from_str(&body).map_err(|_| (StatusCode::BAD_REQUEST, ""))?;
        if let Some(gallons) = units.gallons {
            if units.liters.is_some()
                || units.litres.is_some()
                || units.pints.is_some()
                || !units.extra.is_empty()
            {
                return Err((StatusCode::BAD_REQUEST, ""));
            }

            return Ok((
                StatusCode::OK,
                serde_json::to_string(&MilkUnits {
                    liters: Some(gallons * 3.785412),
                    ..Default::default()
                })
                .unwrap(),
            ));
        } else if let Some(liters) = units.liters {
            if units.gallons.is_some()
                || units.litres.is_some()
                || units.pints.is_some()
                || !units.extra.is_empty()
            {
                return Err((StatusCode::BAD_REQUEST, ""));
            }

            return Ok((
                StatusCode::OK,
                serde_json::to_string(&MilkUnits {
                    gallons: Some(liters / 3.785412),
                    ..Default::default()
                })
                .unwrap(),
            ));
        } else if let Some(litres) = units.litres {
            if units.gallons.is_some()
                || units.liters.is_some()
                || units.pints.is_some()
                || !units.extra.is_empty()
            {
                return Err((StatusCode::BAD_REQUEST, ""));
            }

            return Ok((
                StatusCode::OK,
                serde_json::to_string(&MilkUnits {
                    pints: Some(litres * 1.759754),
                    ..Default::default()
                })
                .unwrap(),
            ));
        } else if let Some(pints) = units.pints {
            if units.gallons.is_some()
                || units.liters.is_some()
                || units.litres.is_some()
                || !units.extra.is_empty()
            {
                return Err((StatusCode::BAD_REQUEST, ""));
            }

            return Ok((
                StatusCode::OK,
                serde_json::to_string(&MilkUnits {
                    litres: Some(pints / 1.759754),
                    ..Default::default()
                })
                .unwrap(),
            ));
        } else {
            return Err((StatusCode::BAD_REQUEST, ""));
        }
    }

    Ok((StatusCode::OK, "Milk withdrawn\n".to_string()))
}

pub async fn refill(State(state): State<Milk>) -> impl IntoResponse {
    let mut limiter = state.limiter.lock().unwrap();
    *limiter = new_limiter();

    StatusCode::OK
}
