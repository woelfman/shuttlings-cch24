use axum::Router;
use shuttle_runtime::CustomError;
use shuttlings_cch24::day;
use sqlx::PgPool;

#[shuttle_runtime::main]
async fn main(#[shuttle_shared_db::Postgres] pool: PgPool) -> shuttle_axum::ShuttleAxum {
    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(CustomError::new)?;

    let router = Router::new()
        .merge(day::d_1::get_routes())
        .merge(day::d2::get_routes())
        .merge(day::d5::get_routes())
        .merge(day::d9::get_routes())
        .merge(day::d12::get_routes())
        .merge(day::d16::get_routes())
        .merge(day::d19::get_routes(pool.clone()))
        .merge(day::d23::get_routes());

    Ok(router.into())
}
