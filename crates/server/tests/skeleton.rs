//! The ground the rest of the server stands on: it opens its database, it
//! answers a health check, and it can be pointed somewhere other than the
//! defaults.

use std::net::{IpAddr, Ipv4Addr};

use askance_server::{Config, open_database, router};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use clap::Parser;
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn opening_the_database_creates_a_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state/askance.db");
    assert!(!path.exists());

    let _pool = open_database(&path).await.unwrap();

    assert!(path.exists(), "expected {} to be created", path.display());
}

#[tokio::test]
async fn opening_the_database_reuses_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("askance.db");

    let pool = open_database(&path).await.unwrap();
    sqlx::query("CREATE TABLE marker (id INTEGER PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let pool = open_database(&path).await.unwrap();
    sqlx::query("SELECT id FROM marker")
        .fetch_optional(&pool)
        .await
        .expect("reopening the database should find the existing table");
}

#[tokio::test]
async fn health_route_answers_ok() {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("askance.db")).await.unwrap();

    let response = router(pool)
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}

#[test]
fn config_defaults_to_localhost() {
    let config = Config::parse_from(["askance serve"]);

    assert_eq!(config.listen.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert!(!config.database.as_os_str().is_empty());
}

#[test]
fn config_is_overridable_by_flag() {
    let config = Config::parse_from([
        "askance serve",
        "--listen",
        "0.0.0.0:9999",
        "--database",
        "/srv/askance/state.db",
    ]);

    assert_eq!(config.listen.to_string(), "0.0.0.0:9999");
    assert_eq!(config.database.to_str().unwrap(), "/srv/askance/state.db");
}
