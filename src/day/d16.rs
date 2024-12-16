use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde_json::Value;

pub fn get_routes() -> Router {
    Router::new()
        .route("/16/wrap", post(wrap))
        .route("/16/unwrap", get(unwrap))
        .route("/16/decode", post(decode_))
}

async fn wrap(headers: HeaderMap, body: String) -> Result<Response, impl IntoResponse> {
    if !matches!(
        headers
            .get(header::CONTENT_TYPE)
            .map(|header| header.as_bytes()),
        Some(b"application/json")
    ) {
        return Err((StatusCode::BAD_REQUEST, "".into()));
    }

    let msg = serde_json::from_str::<Value>(&body).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to deserialize JSON: {e}"),
        )
    })?;
    let token = encode(
        &Header::default(),
        &msg,
        &EncodingKey::from_secret("secret".as_ref()),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate token: {e}"),
        )
    })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::SET_COOKIE, format!("gift={token}"))
        .body("".into())
        .unwrap())
}

async fn unwrap(headers: HeaderMap) -> Result<Response, (StatusCode, String)> {
    let cookie = headers
        .get("Cookie")
        .map(|header| header.to_str())
        .transpose()
        .map_or_else(
            |_| Err((StatusCode::BAD_REQUEST, "".to_string())),
            |res| res.ok_or((StatusCode::BAD_REQUEST, "".to_string())),
        )?;

    let cookie = cookie
        .strip_prefix("gift=")
        .ok_or((StatusCode::BAD_REQUEST, "".to_string()))?;

    let mut validation = Validation::default();
    validation.validate_exp = false; // Disable expiration check
    validation.required_spec_claims.clear();

    let token = decode::<Value>(
        cookie,
        &DecodingKey::from_secret("secret".as_ref()),
        &validation,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unable to decode token: {e}"),
        )
    })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(serde_json::to_string(&token.claims).unwrap().into())
        .unwrap())
}

async fn decode_(body: String) -> Result<Response, StatusCode> {
    let decoding_key =
        DecodingKey::from_rsa_pem(include_bytes!("../../day16_santa_public_key.pem")).unwrap();

    let mut validation = Validation::default();
    validation.algorithms = vec![Algorithm::RS256, Algorithm::RS512];
    validation.validate_exp = false;
    validation.required_spec_claims.clear();

    let token = decode::<Value>(&body, &decoding_key, &validation);

    match token {
        Ok(token) => {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(
                    serde_json::to_string(&token.claims)
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                        .into(),
                )
                .unwrap())
        }
        Err(err) => match err.kind() {
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                Err(StatusCode::UNAUTHORIZED)
            }
            _ => Err(StatusCode::BAD_REQUEST),
        },
    }
}
