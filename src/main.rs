use axum::Router;
use shuttlings_cch24::day;

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .merge(day::d_1::get_routes())
        .merge(day::d2::get_routes())
        .merge(day::d5::get_routes())
        .merge(day::d9::get_routes())
        .merge(day::d12::get_routes());

    Ok(router.into())
}
