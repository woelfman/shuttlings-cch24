use std::borrow::Cow;
use std::str::FromStr;

use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;

pub fn get_routes() -> Router {
    Router::new().route("/5/manifest", post(manifest))
}

pub async fn manifest(headers: HeaderMap, body: String) -> Result<String, impl IntoResponse> {
    // Convert the body to a toml string
    let cargo_toml_content: Cow<String> =
        match headers.get(CONTENT_TYPE).map(|header| header.as_bytes()) {
            Some(b"application/toml") => Cow::Borrowed(&body),
            Some(b"application/yaml") => {
                let v = serde_yaml::from_str::<toml::Value>(&body)
                    .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid manifest"))?;
                Cow::Owned(toml::to_string(&v).unwrap())
            }
            Some(b"application/json") => {
                let v = serde_json::from_str::<toml::Value>(&body)
                    .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid manifest"))?;
                Cow::Owned(toml::to_string(&v).unwrap())
            }
            _ => return Err((StatusCode::UNSUPPORTED_MEDIA_TYPE, "")),
        };

    // Parse the manifest
    let manifest = if let Ok(manifest) = cargo_manifest::Manifest::from_str(&cargo_toml_content) {
        manifest
    } else {
        return Err((StatusCode::BAD_REQUEST, "Invalid manifest"));
    };

    // Validate the keyword "Christmas 2024"
    if let Some(true) = manifest
        .package
        .as_ref()
        .and_then(|package| {
            package
                .keywords
                .as_ref()
                .map(|keywords| keywords.as_ref().as_local())
        })
        .flatten()
        .map(|keywords| keywords.iter().any(|keyword| keyword == "Christmas 2024"))
    {
    } else {
        return Err((StatusCode::BAD_REQUEST, "Magic keyword not provided"));
    }

    // Process the orders
    let mut processed_orders: Vec<(String, u32)> = Vec::new();

    if let Some(orders) = manifest
        .package
        .as_ref()
        .and_then(|package| package.metadata.as_ref())
        .and_then(|metadata| metadata.as_table())
        .and_then(|metadata| metadata.get("orders"))
        .and_then(|orders| orders.as_array())
    {
        orders.iter().for_each(|order| {
            if let Some(table) = order.as_table() {
                if let (Some(item), Some(quantity)) = (
                    table.get("item").and_then(|item| item.as_str()),
                    table
                        .get("quantity")
                        .and_then(|quantity| quantity.as_integer()),
                ) {
                    processed_orders.push((item.to_string(), quantity as u32));
                }
            }
        });
    }

    // Serialize a response
    let response = processed_orders
        .iter()
        .map(|(item, quantity)| format!("{item}: {quantity}"))
        .collect::<Vec<_>>()
        .join("\n");

    if response.is_empty() {
        return Err((StatusCode::NO_CONTENT, ""));
    }

    Ok(response)
}
