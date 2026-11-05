//! Public-client Supabase Auth transport, shared by the desktop and WASM frontends.
//!
//! Credentials are sent only to a configured hosted Supabase project over HTTPS. Never log
//! requests, responses or sessions: all three can contain passwords or bearer tokens.

use serde_json::{Value, json};
use std::time::Duration;

const MAX_RESPONSE: usize = 64 * 1024;
const MAX_PASSWORD: usize = 4096;
const MAX_TOKEN: usize = 16 * 1024;
const INVALID_RESPONSE: &str = "O serviço de contas retornou uma resposta inválida. Tente novamente.";

/// Only public configuration belongs in an installed application.
#[derive(Clone)]
pub struct Config {
    base_url: String,
    publishable_key: String,
}

impl Config {
    /// Accept hosted project origins only, with no credentials, port, path or query string.
    /// This intentionally excludes custom domains and local development Auth servers.
    pub fn new(url: &str, key: &str) -> Result<Self, String> {
        let url = url.trim();
        let url = url.strip_suffix('/').unwrap_or(url);
        let project = url.strip_prefix("https://").and_then(|host| host.strip_suffix(".supabase.co"));
        let valid_project = project.is_some_and(|name| {
            !name.is_empty()
                && name.len() <= 63
                && !name.starts_with('-')
                && !name.ends_with('-')
                && name.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        });
        if !valid_project {
            return Err("Informe a URL HTTPS do projeto Supabase, sem caminhos ou parâmetros.".into());
        }
        let key = key.trim();
        let valid_key = key.strip_prefix("sb_publishable_").is_some_and(|value| {
            (16..=512).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        });
        if !valid_key {
            return Err("Use somente a chave pública sb_publishable_ do projeto. Chaves secretas não são aceitas.".into());
        }
        Ok(Self { base_url: url.to_owned(), publishable_key: key.to_owned() })
    }
}

/// Tokens are intentionally not Debug or automatically persisted.
#[derive(Clone)]
pub struct AuthSession {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub email: String,
    /// Unix seconds; callers refresh before this deadline and stop using an expired session.
    pub expires_at: u64,
}

/// Actions are intentionally not Debug: several variants carry credentials.
pub enum Action {
    SignIn {
        email: String,
        password: String,
    },
    SignUp {
        email: String,
        password: String,
    },
    SendRecovery {
        email: String,
    },
    VerifyRecovery {
        email: String,
        token: String,
    },
    /// A copied, unopened default recovery email link. Never navigate to the supplied URL.
    VerifyRecoveryLink {
        link: String,
    },
    VerifySignup {
        email: String,
        token: String,
    },
    UpdatePassword {
        access_token: String,
        password: String,
    },
    Refresh {
        refresh_token: String,
    },
    SignOut {
        access_token: String,
    },
}

pub enum Outcome {
    /// A recovery session authorizes password replacement, not entry into the editor. The UI
    /// must distinguish VerifyRecovery from SignIn/VerifySignup when consuming this outcome.
    Session(AuthSession),
    Message(String),
    SignedOut,
}

#[derive(Clone, Copy)]
enum Expected {
    Session,
    Registration,
    RecoverySent,
    PasswordUpdated,
    SignedOut,
}

/// Start a non-blocking request. The callback may run on a worker thread; use a channel to
/// return to the UI and request a repaint there. Errors never contain server text or secrets.
pub fn send(config: &Config, action: Action, callback: impl FnOnce(Result<Outcome, String>) + Send + 'static) {
    let (request, expected) = match request(config, action) {
        Ok(request) => request,
        Err(error) => {
            callback(Err(error));
            return;
        }
    };
    ehttp::fetch(request, move |result| {
        let outcome = match result {
            Ok(response) => handle_response(expected, response.status, &response.bytes, unix_seconds()),
            Err(_) => Err("Não foi possível conectar ao serviço de contas. Verifique sua conexão e tente novamente.".into()),
        };
        callback(outcome);
    });
}

/// Exposed for the UI's expiry checks; no calendar conversion is performed here.
pub fn unix_seconds() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::SystemTime::UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs())
}

fn request(config: &Config, action: Action) -> Result<(ehttp::Request, Expected), String> {
    let (path, body, expected, bearer, put) = match action {
        Action::SignIn { email, password } => {
            validate_credentials(&email, &password, false)?;
            ("token?grant_type=password", json!({"email":email.trim(),"password":password}), Expected::Session, None, false)
        }
        Action::SignUp { email, password } => {
            validate_credentials(&email, &password, true)?;
            ("signup", json!({"email":email.trim(),"password":password}), Expected::Registration, None, false)
        }
        Action::SendRecovery { email } => {
            validate_email(&email)?;
            ("recover", json!({"email":email.trim()}), Expected::RecoverySent, None, false)
        }
        Action::VerifyRecovery { email, token } => verify_request(email, token, "recovery")?,
        Action::VerifyRecoveryLink { link } => recovery_link_request(config, &link)?,
        Action::VerifySignup { email, token } => verify_request(email, token, "signup")?,
        Action::UpdatePassword { access_token, password } => {
            validate_password(&password, true)?;
            validate_token(&access_token)?;
            ("user", json!({"password":password}), Expected::PasswordUpdated, Some(access_token), true)
        }
        Action::Refresh { refresh_token } => {
            validate_token(&refresh_token)?;
            ("token?grant_type=refresh_token", json!({"refresh_token":refresh_token}), Expected::Session, None, false)
        }
        Action::SignOut { access_token } => {
            validate_token(&access_token)?;
            // Local scope revokes this session's refresh token without logging other devices out.
            ("logout?scope=local", json!({}), Expected::SignedOut, Some(access_token), false)
        }
    };
    let bytes = serde_json::to_vec(&body).map_err(|_| "Não foi possível preparar a solicitação.".to_owned())?;
    let url = format!("{}/auth/v1/{path}", config.base_url);
    let mut request = if put { ehttp::Request::put(url, bytes) } else { ehttp::Request::post(url, bytes) }
        .with_headers(ehttp::Headers::new(&[
            ("apikey", config.publishable_key.as_str()),
            ("Content-Type", "application/json"),
            ("Accept", "application/json"),
        ]))
        .with_timeout(Some(Duration::from_secs(25)));
    if let Some(token) = bearer {
        request = request.with_header("Authorization", format!("Bearer {token}"));
    }
    Ok((request, expected))
}

type RequestParts = (&'static str, Value, Expected, Option<String>, bool);

fn recovery_link_request(config: &Config, link: &str) -> Result<RequestParts, String> {
    const INVALID_LINK: &str = "Cole o link de recuperação completo deste aplicativo, copiado do e-mail sem abri-lo.";
    if link.len() > 4096 {
        return Err(INVALID_LINK.into());
    }
    let link = link.trim();
    if !link.bytes().all(|byte| byte.is_ascii_graphic()) || link.contains('#') {
        return Err(INVALID_LINK.into());
    }
    // The prefix includes the exact origin AND path terminator. A lookalike host, user-info,
    // alternate port, fragment or encoded path cannot be interpreted as this project's URL.
    let prefix = format!("{}/auth/v1/verify?", config.base_url);
    let query = link.strip_prefix(&prefix).ok_or_else(|| INVALID_LINK.to_owned())?;
    let mut token = None;
    let mut kind = None;
    let mut redirect_seen = false;
    for part in query.split('&') {
        let (name, value) = part.split_once('=').ok_or_else(|| INVALID_LINK.to_owned())?;
        match name {
            "token" if token.is_none() => token = Some(value),
            "type" if kind.is_none() => kind = Some(value),
            "redirect_to" if !redirect_seen => redirect_seen = true,
            _ => return Err(INVALID_LINK.into()),
        }
    }
    let token = token.filter(|token| matches!(token.len(), 56 | 64) && token.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if kind != Some("recovery") || token.is_none() {
        return Err(INVALID_LINK.into());
    }
    // Hosted Auth currently hashes OTPs with SHA-224 (56 hexadecimal digits). Accept SHA-256
    // too for compatibility. No percent decoding is needed for these unescaped hex tokens.
    let token = token.ok_or_else(|| INVALID_LINK.to_owned())?;
    Ok(("verify", json!({"token_hash":token,"type":"recovery"}), Expected::Session, None, false))
}

fn verify_request(email: String, token: String, kind: &'static str) -> Result<RequestParts, String> {
    validate_email(&email)?;
    let token = token.trim();
    if !(6..=10).contains(&token.len()) || !token.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("Informe o código numérico enviado para seu e-mail.".into());
    }
    Ok(("verify", json!({"email":email.trim(),"token":token,"type":kind}), Expected::Session, None, false))
}

fn validate_email(email: &str) -> Result<(), String> {
    let email = email.trim();
    let valid = email.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.') && !domain.contains('@')
    });
    if email.len() > 254 || !valid || email.chars().any(|ch| ch.is_whitespace() || ch.is_control()) {
        return Err("Informe um endereço de e-mail válido.".into());
    }
    Ok(())
}

fn validate_password(password: &str, new_password: bool) -> Result<(), String> {
    if password.is_empty() || password.len() > MAX_PASSWORD {
        return Err("Informe uma senha com até 4.096 bytes.".into());
    }
    if new_password && password.chars().count() < 8 {
        return Err("A senha deve ter pelo menos 8 caracteres.".into());
    }
    Ok(())
}

fn validate_credentials(email: &str, password: &str, new_password: bool) -> Result<(), String> {
    validate_email(email)?;
    validate_password(password, new_password)
}

fn validate_token(token: &str) -> Result<(), String> {
    if token.is_empty() || token.len() > MAX_TOKEN || !token.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err("Sua sessão não é válida. Entre novamente.".into());
    }
    Ok(())
}

fn handle_response(expected: Expected, status: u16, bytes: &[u8], now: u64) -> Result<Outcome, String> {
    // ehttp buffers the network response before this callback. This cap bounds JSON parsing,
    // not the transport's allocation; hosted Supabase is the only allowed origin.
    if bytes.len() > MAX_RESPONSE {
        return Err(INVALID_RESPONSE.into());
    }
    if !(200..300).contains(&status) {
        return Err(server_error(status, bytes));
    }
    match expected {
        Expected::SignedOut => Ok(Outcome::SignedOut),
        Expected::RecoverySent => {
            Ok(Outcome::Message("Se houver uma conta com esse e-mail, você receberá instruções para redefinir a senha.".into()))
        }
        Expected::PasswordUpdated => {
            let user: Value = serde_json::from_slice(bytes).map_err(|_| INVALID_RESPONSE.to_owned())?;
            if user.get("id").and_then(Value::as_str).is_none_or(str::is_empty) {
                return Err(INVALID_RESPONSE.into());
            }
            Ok(Outcome::Message("Senha atualizada. Entre com sua nova senha.".into()))
        }
        Expected::Session | Expected::Registration => {
            let value: Value = serde_json::from_slice(bytes).map_err(|_| INVALID_RESPONSE.to_owned())?;
            if matches!(expected, Expected::Registration) && value.get("access_token").is_none() {
                if value.get("id").and_then(Value::as_str).is_none_or(str::is_empty) {
                    return Err(INVALID_RESPONSE.into());
                }
                return Ok(Outcome::Message(
                    "Confira seu e-mail para confirmar o cadastro. Se já tiver uma conta, entre ou recupere sua senha.".into(),
                ));
            }
            session(&value, now).map(Outcome::Session)
        }
    }
}

fn session(value: &Value, now: u64) -> Result<AuthSession, String> {
    let field = |name: &str| value.get(name).and_then(Value::as_str).ok_or_else(|| INVALID_RESPONSE.to_owned());
    let access_token = field("access_token")?;
    let refresh_token = field("refresh_token")?;
    validate_token(access_token).map_err(|_| INVALID_RESPONSE.to_owned())?;
    validate_token(refresh_token).map_err(|_| INVALID_RESPONSE.to_owned())?;
    if !field("token_type")?.eq_ignore_ascii_case("bearer") {
        return Err(INVALID_RESPONSE.into());
    }
    let expires_in = value.get("expires_in").and_then(Value::as_u64).filter(|seconds| (1..=86400).contains(seconds));
    let expires_at = expires_in.and_then(|seconds| now.checked_add(seconds)).ok_or_else(|| INVALID_RESPONSE.to_owned())?;
    let user = value.get("user").ok_or_else(|| INVALID_RESPONSE.to_owned())?;
    let email = user.get("email").and_then(Value::as_str).ok_or_else(|| INVALID_RESPONSE.to_owned())?;
    let user_id = user.get("id").and_then(Value::as_str).filter(|id| !id.is_empty() && id.len() <= 128);
    validate_email(email).map_err(|_| INVALID_RESPONSE.to_owned())?;
    Ok(AuthSession {
        access_token: access_token.to_owned(),
        refresh_token: refresh_token.to_owned(),
        user_id: user_id.ok_or_else(|| INVALID_RESPONSE.to_owned())?.to_owned(),
        email: email.to_owned(),
        expires_at,
    })
}

fn server_error(status: u16, bytes: &[u8]) -> String {
    let value: Option<Value> = serde_json::from_slice(bytes).ok();
    let code = value.as_ref().and_then(|value| value.get("error_code")).and_then(Value::as_str).unwrap_or("");
    let message = match (status, code) {
        (429, _) | (_, "over_email_send_rate_limit" | "over_request_rate_limit") => "Muitas tentativas. Aguarde um pouco antes de tentar novamente.",
        (_, "email_not_confirmed") => "Confirme seu e-mail antes de entrar.",
        (_, "weak_password" | "same_password") => "Escolha uma senha diferente e mais forte, com pelo menos 8 caracteres.",
        (_, "otp_expired" | "otp_disabled") => "O código expirou ou não é válido. Solicite um novo código.",
        (_, "signup_disabled" | "email_provider_disabled") => "O cadastro por e-mail está desativado neste serviço.",
        (_, "captcha_failed") => "O serviço exige uma verificação adicional. Entre em contato com o responsável pelo aplicativo.",
        (401 | 403, _) => "Acesso recusado. Confira seus dados ou entre novamente.",
        (400 | 422, _) => "Não foi possível concluir. Confira o e-mail, a senha ou o código informado.",
        (500..=599, _) => "O serviço de contas está indisponível. Tente novamente em alguns instantes.",
        _ => "Não foi possível concluir a solicitação. Tente novamente.",
    };
    message.into()
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
