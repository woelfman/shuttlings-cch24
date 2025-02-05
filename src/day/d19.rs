use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{self, Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use rand::{distr::Alphanumeric, Rng};
use sea_query::{ColumnDef, Expr, Iden, Order, PostgresQueryBuilder, Query, Table};
use sea_query_binder::SqlxBinder;
use serde::{Deserialize, Serialize};
use sqlx::{
    prelude::FromRow,
    types::chrono::{self, DateTime},
    PgPool,
};
use uuid::Uuid;

pub fn get_routes(pool: PgPool) -> Router {
    let state = AppState {
        pool,
        tokens: Arc::new(Mutex::new(HashMap::new())),
    };

    Router::new()
        .route("/19/reset", post(reset))
        .route("/19/cite/{id}", get(cite))
        .route("/19/remove/{id}", delete(remove))
        .route("/19/undo/{id}", put(undo))
        .route("/19/draft", post(draft))
        .route("/19/list", get(list))
        .with_state(state)
}

#[derive(Clone)]
struct AppState {
    tokens: Arc<Mutex<HashMap<String, i32>>>,
    pool: PgPool,
}

#[derive(Iden)]
enum Quotes {
    Table,
    Id,
    Author,
    Quote,
    CreatedAt,
    Version,
}

async fn reset(State(state): State<AppState>) -> Result<StatusCode, StatusCode> {
    let query = Table::drop()
        .table(Quotes::Table)
        .if_exists()
        .build(PostgresQueryBuilder);

    sqlx::query(&query)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let query = Table::create()
        .table(Quotes::Table)
        .if_not_exists()
        .col(ColumnDef::new(Quotes::Id).uuid().primary_key())
        .col(ColumnDef::new(Quotes::Author).text().not_null())
        .col(ColumnDef::new(Quotes::Quote).text().not_null())
        .col(
            ColumnDef::new(Quotes::CreatedAt)
                .timestamp_with_time_zone()
                .not_null()
                .default(Expr::current_timestamp()),
        )
        .col(
            ColumnDef::new(Quotes::Version)
                .integer()
                .not_null()
                .default(1),
        )
        .build(PostgresQueryBuilder);

    sqlx::query(&query)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
struct QuoteStruct {
    id: Uuid,
    author: String,
    quote: String,
    created_at: DateTime<chrono::Utc>,
    version: i32,
}

async fn cite(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<QuoteStruct>, StatusCode> {
    let (sql, values) = Query::select()
        .columns([
            Quotes::Id,
            Quotes::Author,
            Quotes::Quote,
            Quotes::CreatedAt,
            Quotes::Version,
        ])
        .from(Quotes::Table)
        .and_where(Expr::col(Quotes::Id).eq(id))
        .build_sqlx(PostgresQueryBuilder);

    let quote = sqlx::query_as_with::<_, QuoteStruct, _>(&sql, values)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(quote))
}

async fn remove(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<QuoteStruct>, StatusCode> {
    let (sql, values) = Query::delete()
        .from_table(Quotes::Table)
        .and_where(Expr::col(Quotes::Id).eq(id))
        .returning_all()
        .build_sqlx(PostgresQueryBuilder);

    let result = sqlx::query_as_with::<_, QuoteStruct, _>(&sql, values)
        .fetch_optional(&state.pool)
        .await;

    if let Ok(Some(quote)) = result {
        return Ok(Json(quote));
    }

    Err(StatusCode::NOT_FOUND)
}

#[derive(Deserialize)]
struct Undo {
    author: Option<String>,
    quote: Option<String>,
}

async fn undo(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(quote): Json<Undo>,
) -> Result<Json<QuoteStruct>, StatusCode> {
    let mut query = Query::update();
    query
        .table(Quotes::Table)
        .and_where(Expr::col(Quotes::Id).eq(id))
        .returning_all();

    if let Some(author) = quote.author {
        query.value(Quotes::Author, Expr::val(author));
    }

    if let Some(quote) = quote.quote {
        query.value(Quotes::Quote, Expr::value(quote));
    }

    query.value(Quotes::Version, Expr::col(Quotes::Version).add(1));

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

    let quote = sqlx::query_as_with::<_, QuoteStruct, _>(&sql, values)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(quote))
}

#[derive(Deserialize)]
struct Draft {
    author: String,
    quote: String,
}

async fn draft(
    State(state): State<AppState>,
    Json(draft): Json<Draft>,
) -> Result<(StatusCode, Json<QuoteStruct>), StatusCode> {
    let (sql, values) = Query::insert()
        .into_table(Quotes::Table)
        .columns([Quotes::Id, Quotes::Author, Quotes::Quote])
        .values_panic([
            Uuid::new_v4().into(),
            draft.author.into(),
            draft.quote.into(),
        ])
        .returning_all()
        .build_sqlx(PostgresQueryBuilder);

    let quote = sqlx::query_as_with::<_, QuoteStruct, _>(&sql, values)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok((StatusCode::CREATED, Json(quote)))
}

#[derive(Deserialize)]
struct PageQuery {
    token: Option<String>,
}

fn generate_token() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

#[derive(Serialize, Default)]
struct PageResponse {
    quotes: Vec<QuoteStruct>,
    page: u32,
    next_token: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    extract::Query(params): extract::Query<PageQuery>,
) -> Result<Json<PageResponse>, StatusCode> {
    let page = match &params.token {
        Some(token) => {
            if let Some(token) = state.tokens.lock().unwrap().get(token).copied() {
                token
            } else {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        None => 1,
    };

    let limit = 3;
    let offset = (page - 1) * limit;

    let (sql, values) = Query::select()
        .columns([
            Quotes::Id,
            Quotes::Author,
            Quotes::Quote,
            Quotes::CreatedAt,
            Quotes::Version,
        ])
        .from(Quotes::Table)
        .order_by(Quotes::CreatedAt, Order::Asc)
        .limit(limit as u64 + 1)
        .offset(offset as u64)
        .build_sqlx(PostgresQueryBuilder);

    let mut quotes = sqlx::query_as_with::<_, QuoteStruct, _>(&sql, values)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let next_token = {
        let mut tokens = state.tokens.lock().unwrap();

        if quotes.len() == limit as usize + 1 {
            let next_page = page + 1;
            let token = generate_token();
            tokens.insert(token.clone(), next_page);
            Some(token)
        } else {
            quotes.truncate(limit as usize);
            params.token.as_ref().map(|token| tokens.remove(token));
            None
        }
    };

    Ok(Json(PageResponse {
        quotes,
        page: page as u32,
        next_token,
    }))
}
