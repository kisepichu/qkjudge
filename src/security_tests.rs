use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use actix_web::{http::StatusCode, test, web, App, HttpResponse};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

use crate::routes;

fn identity() -> IdentityService<CookieIdentityPolicy> {
    IdentityService::new(
        CookieIdentityPolicy::new(&[42; 32])
            .name("auth")
            .secure(true),
    )
}

fn unavailable_pool() -> web::Data<Arc<Mutex<sqlx::MySqlPool>>> {
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_millis(20))
        .connect_lazy("mysql://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    web::Data::new(Arc::new(Mutex::new(pool)))
}

#[actix_web::test]
async fn anonymous_execution_is_rejected_before_compiler_access() {
    let app = test::init_service(
        App::new()
            .wrap(identity())
            .service(routes::post_execute_handler),
    )
    .await;
    let request = test::TestRequest::post()
        .uri("/execute")
        .set_json(json!({"language_id": 0, "source": "", "input": ""}))
        .to_request();
    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn public_signup_cannot_claim_privileged_identities() {
    let app = test::init_service(
        App::new()
            .app_data(unavailable_pool())
            .service(routes::post_signup_handler),
    )
    .await;
    for username in ["admin", "tqk", "AdMiN", "TQK"] {
        let request = test::TestRequest::post()
            .uri("/user/signup")
            .set_json(json!({"username": username, "password": "test-password"}))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{username}");
    }
}

#[actix_web::test]
async fn passwords_above_bcrypt_byte_limit_are_rejected_before_database_access() {
    let app = test::init_service(
        App::new()
            .app_data(unavailable_pool())
            .wrap(identity())
            .service(routes::post_signup_handler)
            .service(routes::post_login_handler),
    )
    .await;
    for endpoint in ["/user/signup", "/user/login"] {
        let request = test::TestRequest::post()
            .uri(endpoint)
            .set_json(json!({"username": "alice", "password": "あ".repeat(25)}))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{endpoint}");
    }
}

#[actix_web::test]
async fn malformed_webhook_header_is_rejected_without_panicking() {
    let app = test::init_service(
        App::new()
            .app_data(unavailable_pool())
            .wrap(identity())
            .service(routes::post_fetch_problems_handler),
    )
    .await;
    let request = test::TestRequest::post()
        .uri("/fetch/problems")
        .insert_header(("X-Hub-Signature-256", "x"))
        .insert_header(("X-Hub-Signature", "x"))
        .set_payload([0xff, 0xfe].as_slice())
        .to_request();
    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[actix_web::test]
async fn invalid_language_is_rejected_for_authenticated_execution() {
    let app = test::init_service(
        App::new()
            .wrap(identity())
            .service(web::resource("/test-login").to(|id: Identity| async move {
                id.remember("alice".to_owned());
                HttpResponse::Ok().finish()
            }))
            .service(routes::post_execute_handler),
    )
    .await;
    let login = test::call_service(
        &app,
        test::TestRequest::get().uri("/test-login").to_request(),
    )
    .await;
    let cookie = login.response().cookies().next().unwrap().into_owned();
    for language_id in [-1, 267, i32::MAX] {
        let request = test::TestRequest::post()
            .uri("/execute")
            .cookie(cookie.clone())
            .set_json(json!({"language_id": language_id, "source": "x", "input": ""}))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{language_id}");
    }
}

#[actix_web::test]
async fn problem_registration_requires_administrator_identity() {
    let app = test::init_service(
        App::new()
            .app_data(unavailable_pool())
            .wrap(identity())
            .service(routes::post_problems_handler),
    )
    .await;
    let request = test::TestRequest::post()
        .uri("/problems")
        .set_json(json!({"path": "../outside"}))
        .to_request();
    assert_eq!(
        test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );
}

#[actix_web::test]
async fn cross_site_logout_is_rejected_and_trusted_logout_is_allowed() {
    let app = test::init_service(
        App::new()
            .wrap(identity())
            .wrap(crate::security::cors())
            .service(routes::post_logout_handler),
    )
    .await;
    for origin in ["https://attacker.invalid", "null"] {
        let request = test::TestRequest::post()
            .uri("/user/logout")
            .insert_header(("Origin", origin))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!response.headers().contains_key("Set-Cookie"));
    }
    let request = test::TestRequest::post().uri("/user/logout").to_request();
    assert_eq!(
        test::call_service(&app, request).await.status(),
        StatusCode::FORBIDDEN
    );
    let request = test::TestRequest::post()
        .uri("/user/logout")
        .insert_header(("Origin", crate::security::FRONTEND_ORIGIN))
        .to_request();
    assert_eq!(
        test::call_service(&app, request).await.status(),
        StatusCode::NO_CONTENT
    );
}
