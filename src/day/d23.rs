use axum::{
    extract::{Multipart, Path},
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Router,
};
use handlebars::Handlebars;
use serde::Deserialize;
use serde_json::json;
use tower_http::services::ServeDir;

pub fn get_routes() -> Router {
    Router::new()
        .nest_service("/assets/", ServeDir::new("assets"))
        .route("/23/star", get(star))
        .route("/23/present/{color}", get(present))
        .route("/23/ornament/{state}/{n}", get(ornament))
        .route("/23/lockfile", post(lockfile))
}

// HTMX handler to light up the star
async fn star() -> Html<&'static str> {
    // Replace the #star element with the lit version
    Html(r#"<div id="star" class="lit"></div>"#)
}

async fn present(Path(color): Path<String>) -> Result<Html<String>, StatusCode> {
    let next_color = match color.as_ref() {
        "red" => "blue",
        "blue" => "purple",
        "purple" => "red",
        _ => return Err(StatusCode::IM_A_TEAPOT),
    };

    let html = format!(
        r#"
        <div class="present {color}" hx-get="/23/present/{next_color}" hx-swap="outerHTML">
            <div class="ribbon"></div>
            <div class="ribbon"></div>
            <div class="ribbon"></div>
            <div class="ribbon"></div>
        </div>
        "#,
        color = color,
        next_color = next_color
    );
    Ok(Html(html))
}

async fn ornament(Path((state, n)): Path<(String, String)>) -> Result<Html<String>, StatusCode> {
    const VALID_STATES: &[&str] = &["on", "off"];

    if !VALID_STATES.contains(&state.as_str()) {
        return Err(StatusCode::IM_A_TEAPOT);
    }

    let next_state = if state == "on" { "off" } else { "on" };

    let class = if state == "on" {
        "ornament on"
    } else {
        "ornament"
    };

    let handlebars = Handlebars::new();

    let template = r#"
        <div class="{{class}}" id="ornament{{n}}" hx-trigger="load delay:2s once" hx-get="/23/ornament/{{next_state}}/{{n}}" hx-swap="outerHTML">
        </div>
    "#;

    let mut ctx = serde_json::Map::new();
    ctx.insert("class".to_string(), json!(class));
    ctx.insert("n".to_string(), json!(n));
    ctx.insert("next_state".to_string(), json!(next_state));

    let html = handlebars
        .render_template(template, &ctx)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html))
}

#[derive(Deserialize)]
struct Lockfile {
    package: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    checksum: Option<String>,
}

async fn lockfile(mut multipart: Multipart) -> Result<Html<String>, StatusCode> {
    let mut content = String::new();
    let mut checksum_elements = Vec::new();

    while let Ok(Some(part)) = multipart.next_field().await {
        if part.name() == Some("lockfile") {
            let part = part.text().await.unwrap();
            content.push_str(&part);
        }
    }

    let lockfile: Lockfile = match toml::de::from_str(&content) {
        Ok(parsed) => parsed,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    for package in lockfile.package {
        if let Some(checksum) = package.checksum {
            if checksum.len() < 6 {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }

            let color = match u32::from_str_radix(&checksum[..6], 16) {
                Ok(c) => format!("#{:06x}", c),
                Err(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
            };

            let top = match u8::from_str_radix(&checksum[6..8], 16) {
                Ok(t) => t,
                Err(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
            };

            let left = match u8::from_str_radix(&checksum[8..10], 16) {
                Ok(l) => l,
                Err(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
            };

            // Prepare the HTML div element with styles
            let div_element = format!(
                r#"<div style="background-color:{color};top:{top}px;left:{left}px;"></div>"#,
                color = color,
                top = top,
                left = left
            );
            checksum_elements.push(div_element);
        }
    }

    Ok(Html(checksum_elements.join("\n")))
}
