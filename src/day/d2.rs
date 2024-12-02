use std::net::{Ipv4Addr, Ipv6Addr};

use axum::extract::Query;
use axum::{routing::get, Router};
use serde::Deserialize;

pub fn get_routes() -> Router {
    Router::new()
        .route("/2/dest", get(task1::dest))
        .route("/2/key", get(task2::key))
        .route("/2/v6/dest", get(task3::dest))
        .route("/2/v6/key", get(task3::key))
}

mod task1 {
    use super::*;

    #[derive(Deserialize)]
    pub struct Dest {
        from: Ipv4Addr,
        key: Ipv4Addr,
    }

    pub async fn dest(dest: Query<Dest>) -> String {
        std::iter::zip(dest.from.octets().iter(), dest.key.octets().iter())
            .map(|(&from, &key)| from.wrapping_add(key).to_string())
            .collect::<Vec<String>>()
            .join(".")
    }
}

mod task2 {
    use super::*;

    #[derive(Deserialize)]
    pub struct Key {
        from: Ipv4Addr,
        to: Ipv4Addr,
    }

    pub async fn key(key: Query<Key>) -> String {
        std::iter::zip(key.from.octets().iter(), key.to.octets().iter())
            .map(|(&from, &to)| to.wrapping_sub(from).to_string())
            .collect::<Vec<String>>()
            .join(".")
    }
}

mod task3 {
    use super::*;

    #[derive(Deserialize)]
    pub struct Dest {
        from: Ipv6Addr,
        key: Ipv6Addr,
    }

    pub async fn dest(dest: Query<Dest>) -> String {
        let to: [u8; 16] = std::iter::zip(dest.from.octets().iter(), dest.key.octets().iter())
            .map(|(&from, &key)| from ^ key)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let to = Ipv6Addr::from(to);
        to.to_string()
    }

    #[derive(Deserialize)]
    pub struct Key {
        from: Ipv6Addr,
        to: Ipv6Addr,
    }

    pub async fn key(key: Query<Key>) -> String {
        let key: [u8; 16] = std::iter::zip(key.from.octets().iter(), key.to.octets().iter())
            .map(|(&from, &to)| to ^ from)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let key = Ipv6Addr::from(key);
        key.to_string()
    }
}
