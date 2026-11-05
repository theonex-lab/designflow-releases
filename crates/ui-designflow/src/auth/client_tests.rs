use super::{Action, Config, Expected, MAX_RESPONSE, Outcome, handle_response, request, server_error, session};
use serde_json::{Value, json};

const URL: &str = "https://synthetic-project.supabase.co";
const KEY: &str = "sb_publishable_synthetic_test_key_1234";

fn config() -> Config {
    Config::new(URL, KEY).expect("synthetic configuration")
}

fn session_json() -> Value {
    json!({
        "access_token":"synthetic.access.token",
        "refresh_token":"synthetic-refresh-token",
        "token_type":"bearer",
        "expires_in":3600,
        "user":{"id":"synthetic-user","email":"person@example.org"}
    })
}

#[test]
fn configuration_rejects_untrusted_origins_and_secret_keys() {
    for url in [
        "http://synthetic-project.supabase.co",
        "https://synthetic-project.supabase.co.attacker.example",
        "https://synthetic-project.supabase.co@attacker.example",
        "https://attacker.example@synthetic-project.supabase.co",
        "https://synthetic-project.supabase.co:443",
        "https://synthetic-project.supabase.co/auth/v1",
        "https://synthetic-project.supabase.co/?key=secret",
        "https://synthetic-project.supabase.co#fragment",
        "https://synthetic-project.supabase.co//",
        "https://.supabase.co",
        "https://-invalid.supabase.co",
        "https://synthetic\nproject.supabase.co",
    ] {
        assert!(Config::new(url, KEY).is_err(), "invalid origin accepted: {url}");
    }
    for key in ["sb_secret_do_not_embed", "eyJhbGciOiJIUzI1NiJ9.service_role", "", "sb_publishable_short", "sb_publishable_invalid\nheader"] {
        assert!(Config::new(URL, key).is_err());
    }
    let valid = Config::new("https://synthetic-project.supabase.co/", KEY).expect("root slash allowed");
    assert_eq!(valid.base_url, URL);
}

#[test]
fn sign_in_keeps_password_whitespace_and_uses_public_key_without_bearer() {
    let (request, _) = request(&config(), Action::SignIn { email: " person@example.org ".into(), password: " password with spaces ".into() })
        .expect("valid credentials");
    assert_eq!(request.url, format!("{URL}/auth/v1/token?grant_type=password"));
    assert_eq!(request.headers.get("apikey"), Some(KEY));
    assert_eq!(request.headers.get("Authorization"), None);
    assert_eq!(request.headers.get("Content-Type"), Some("application/json"));
    assert_eq!(request.headers.get_all("Content-Type").count(), 1);
    assert!(request.timeout.is_some());
    let body: Value = serde_json::from_slice(&request.body).expect("JSON request");
    assert_eq!(body["email"], "person@example.org");
    assert_eq!(body["password"], " password with spaces ");
    assert!(!request.url.contains("password with spaces"));
}

#[test]
fn validation_rejects_oversized_credentials_and_otp_before_network() {
    assert!(request(&config(), Action::SignIn { email: "person@example.org".into(), password: "x".repeat(4097) }).is_err());
    assert!(request(&config(), Action::SignUp { email: "person@example.org".into(), password: "short".into() }).is_err());
    for email in ["missing-at", "person@@example.org", "person@example.org\nAuthorization: token", "@example.org"] {
        assert!(request(&config(), Action::SendRecovery { email: email.into() }).is_err());
    }
    for token in ["12345", "12345678901", "１２３４５６", "123\n56", "12345a"] {
        assert!(request(&config(), Action::VerifyRecovery { email: "person@example.org".into(), token: token.into() }).is_err());
    }
}

#[test]
fn verification_posts_email_code_and_correct_type_without_url_secrets() {
    for (action, kind) in [
        (Action::VerifyRecovery { email: "person@example.org".into(), token: "123456".into() }, "recovery"),
        (Action::VerifySignup { email: "person@example.org".into(), token: "123456".into() }, "signup"),
    ] {
        let (request, _) = request(&config(), action).expect("valid code");
        let body: Value = serde_json::from_slice(&request.body).expect("JSON request");
        assert_eq!(request.url, format!("{URL}/auth/v1/verify"));
        assert_eq!(body["type"], kind);
        assert_eq!(body["token"], "123456");
        assert_eq!(body["email"], "person@example.org");
        assert_eq!(request.headers.get("Authorization"), None);
    }
}

#[test]
fn copied_recovery_link_posts_only_hash_to_configured_endpoint_and_ignores_redirect() {
    for length in [56, 64] {
        let hash = "a".repeat(length);
        let link = format!("{URL}/auth/v1/verify?token={hash}&type=recovery&redirect_to=https%3A%2F%2Funtrusted.example%2F");
        let (request, expected) = request(&config(), Action::VerifyRecoveryLink { link }).expect("valid copied recovery link");
        assert!(matches!(expected, Expected::Session));
        assert_eq!(request.url, format!("{URL}/auth/v1/verify"));
        assert_eq!(request.method, ehttp::Method::POST);
        assert_eq!(request.headers.get("Authorization"), None);
        assert_eq!(request.headers.get("Content-Type"), Some("application/json"));
        assert_eq!(request.headers.get_all("Content-Type").count(), 1);
        let body: Value = serde_json::from_slice(&request.body).expect("JSON request");
        assert_eq!(body, json!({"token_hash":hash,"type":"recovery"}));
    }
}

#[test]
fn copied_recovery_link_rejects_wrong_origin_path_kind_and_duplicate_parameters() {
    let hash = "a".repeat(56);
    let invalid_links = [
        format!("http://synthetic-project.supabase.co/auth/v1/verify?token={hash}&type=recovery"),
        format!("https://other-project.supabase.co/auth/v1/verify?token={hash}&type=recovery"),
        format!("{URL}.untrusted.example/auth/v1/verify?token={hash}&type=recovery"),
        format!("{URL}@untrusted.example/auth/v1/verify?token={hash}&type=recovery"),
        format!("{URL}:443/auth/v1/verify?token={hash}&type=recovery"),
        format!("{URL}/auth/v1/verify/extra?token={hash}&type=recovery"),
        format!("{URL}/auth/v1/verify?token={hash}&type=signup"),
        format!("{URL}/auth/v1/verify?token={hash}&type=recovery&type=recovery"),
        format!("{URL}/auth/v1/verify?token={hash}&type=recovery&token={hash}"),
        format!("{URL}/auth/v1/verify?token={hash}&type=recovery&%74oken={hash}"),
        format!("{URL}/auth/v1/verify?token={hash}&type=recovery&redirect_to=x&redirect_to=y"),
        format!("{URL}/auth/v1/verify?token={hash}&type=recovery#access_token=synthetic"),
        format!("{URL}/auth/v1/verify?token={hash}"),
        format!("{URL}/auth/v1/verify?type=recovery"),
    ];
    for link in invalid_links {
        assert!(request(&config(), Action::VerifyRecoveryLink { link }).is_err());
    }
}

#[test]
fn copied_recovery_link_rejects_invalid_hashes_and_oversized_input_without_echoing_it() {
    for hash in [String::new(), "123456".into(), "a".repeat(55), "a".repeat(57), "a".repeat(65), "g".repeat(56), "%61".repeat(56)] {
        let link = format!("{URL}/auth/v1/verify?token={hash}&type=recovery");
        assert!(request(&config(), Action::VerifyRecoveryLink { link }).is_err());
    }
    let link = format!("{URL}/auth/v1/verify?token={}&type=recovery&redirect_to={}", "a".repeat(56), "sensitive-value".repeat(4096));
    let error = request(&config(), Action::VerifyRecoveryLink { link }).err().expect("oversized link refused");
    assert!(!error.contains("sensitive-value"));
}

#[test]
fn password_update_and_logout_are_authorized_with_user_session_only() {
    let (update, _) = request(&config(), Action::UpdatePassword { access_token: "synthetic-user-token".into(), password: "new-password".into() })
        .expect("valid update");
    assert_eq!(update.url, format!("{URL}/auth/v1/user"));
    assert_eq!(update.method, ehttp::Method::PUT);
    assert_eq!(update.headers.get("Authorization"), Some("Bearer synthetic-user-token"));
    let (logout, _) = request(&config(), Action::SignOut { access_token: "synthetic-user-token".into() }).expect("valid logout");
    assert_eq!(logout.url, format!("{URL}/auth/v1/logout?scope=local"));
    assert_eq!(logout.headers.get("Authorization"), Some("Bearer synthetic-user-token"));
    assert!(request(&config(), Action::SignOut { access_token: "injected\r\nHeader: value".into() }).is_err());
}

#[test]
fn refresh_posts_token_in_body_without_authorization_header() {
    let (request, _) = request(&config(), Action::Refresh { refresh_token: "synthetic-refresh-token".into() }).expect("valid refresh");
    assert_eq!(request.url, format!("{URL}/auth/v1/token?grant_type=refresh_token"));
    assert_eq!(request.headers.get("Authorization"), None);
    assert!(!request.url.contains("synthetic-refresh-token"));
    let body: Value = serde_json::from_slice(&request.body).expect("JSON request");
    assert_eq!(body["refresh_token"], "synthetic-refresh-token");
}

#[test]
fn successful_session_requires_identity_tokens_and_bounded_lifetime() {
    let valid = session_json();
    let parsed = session(&valid, 1000).expect("complete session");
    assert_eq!(parsed.expires_at, 4600);
    assert_eq!(parsed.email, "person@example.org");
    for field in ["access_token", "refresh_token", "token_type", "expires_in", "user"] {
        let mut invalid = valid.clone();
        invalid.as_object_mut().expect("object").remove(field);
        assert!(session(&invalid, 1000).is_err(), "missing {field} accepted");
    }
    for lifetime in [json!(0), json!(-1), json!(86401), json!("3600"), json!(1.5)] {
        let mut invalid = valid.clone();
        invalid["expires_in"] = lifetime;
        assert!(session(&invalid, 1000).is_err());
    }
    assert!(session(&valid, u64::MAX).is_err());
}

#[test]
fn signup_confirmation_never_creates_a_session_without_tokens() {
    let body = br#"{"id":"synthetic-user","email":"person@example.org"}"#;
    assert!(matches!(handle_response(Expected::Registration, 200, body, 1000), Ok(Outcome::Message(_))));
    assert!(handle_response(Expected::Session, 200, body, 1000).is_err());
    assert!(handle_response(Expected::Registration, 200, b"{}", 1000).is_err());
    let body = serde_json::to_vec(&session_json()).expect("session JSON");
    assert!(matches!(handle_response(Expected::Registration, 200, &body, 1000), Ok(Outcome::Session(_))));
}

#[test]
fn response_errors_and_limits_never_echo_server_text_or_secrets() {
    for status in [400, 401, 403, 422, 429, 500, 503] {
        let error = server_error(status, br#"{"error_code":"unknown","msg":"SECRET-PASSWORD synthetic.access.token"}"#);
        assert!(!error.contains("SECRET"));
        assert!(!error.contains("synthetic.access.token"));
    }
    assert!(handle_response(Expected::Session, 200, b"not JSON", 1000).is_err());
    assert!(handle_response(Expected::Session, 200, &vec![b' '; MAX_RESPONSE + 1], 1000).is_err());
    assert!(handle_response(Expected::RecoverySent, 500, b"{}", 1000).is_err());
    assert!(matches!(handle_response(Expected::SignedOut, 204, b"", 1000), Ok(Outcome::SignedOut)));
}
