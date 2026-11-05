//! DesignFlow account entry. Tokens live only in this frontend, never in the document engine.

pub mod client;
#[cfg(test)]
mod local_guard_tests;
mod local_view;
#[cfg(test)]
mod tests;
mod view;

use sha2::{Digest, Sha256};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use client::{Action, AuthSession, Config, Outcome};
use egui::Context;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Screen {
    #[default]
    SignIn,
    SignUp,
    VerifySignup,
    Recovery,
    VerifyRecovery,
    NewPassword,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Operation {
    SignIn,
    SignUp,
    VerifySignup,
    SendRecovery,
    VerifyRecovery,
    UpdatePassword,
    Refresh,
}

struct Pending {
    operation: Operation,
    started: web_time::Instant,
    receiver: Receiver<Result<Outcome, String>>,
}

/// Separate from UiState so every window shares the same account lock.
pub struct Auth {
    config: Result<Config, String>,
    screen: Screen,
    email: String,
    password: String,
    confirmation: String,
    code: String,
    reveal: bool,
    session: Option<AuthSession>,
    recovery: Option<AuthSession>,
    pending: Option<Pending>,
    message: Option<(bool, String)>,
    logo: Option<egui::TextureHandle>,
    email_codes_enabled: bool,
    local_only: bool,
}

impl Auth {
    fn new(config: Result<Config, String>) -> Self {
        Self {
            config,
            screen: Screen::SignIn,
            email: String::new(),
            password: String::new(),
            confirmation: String::new(),
            code: String::new(),
            reveal: false,
            session: None,
            recovery: None,
            pending: None,
            message: None,
            logo: None,
            email_codes_enabled: false,
            local_only: true,
        }
    }

    fn account_file() -> std::path::PathBuf {
        let base = std::env::var_os("APPDATA").map(std::path::PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        base.join("DesignFlow").join("account.json")
    }

    fn digest(salt: &str, email: &str, password: &str) -> String {
        let mut h = Sha256::new();
        h.update(salt.as_bytes());
        h.update(b"\\0");
        h.update(email.trim().to_ascii_lowercase().as_bytes());
        h.update(b"\\0");
        h.update(password.as_bytes());
        format!("{:x}", h.finalize())
    }

    fn local_auth(&mut self, signup: bool, ctx: &Context) {
        if signup && self.password != self.confirmation {
            self.message = Some((true, "As senhas precisam ser iguais.".into()));
            return;
        }
        let email = self.email.trim().to_ascii_lowercase();
        if email.is_empty() || !email.contains('@') {
            self.message = Some((true, "Informe um e-mail válido.".into()));
            return;
        }
        if self.password.chars().count() < 8 {
            self.message = Some((true, "A senha deve ter pelo menos 8 caracteres.".into()));
            return;
        }
        let path = Self::account_file();
        let existing: Option<serde_json::Value> = std::fs::read_to_string(&path).ok().and_then(|text| serde_json::from_str(&text).ok());
        if signup && existing.is_some() {
            self.message = Some((true, "Já existe uma conta neste computador. Entre com ela.".into()));
            return;
        }
        if !signup {
            let Some(account) = existing else {
                self.message = Some((true, "Nenhuma conta local foi criada ainda.".into()));
                return;
            };
            let salt = account["salt"].as_str().unwrap_or_default();
            let expected = account["hash"].as_str().unwrap_or_default();
            if account["email"].as_str().unwrap_or_default() != email || Self::digest(salt, &email, &self.password) != expected {
                self.message = Some((true, "E-mail ou senha incorretos.".into()));
                return;
            }
        } else {
            let salt = format!("{}-{}", client::unix_seconds(), email);
            let data = serde_json::json!({"email": email, "salt": salt, "hash": Self::digest(&salt, &email, &self.password)});
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::write(&path, serde_json::to_vec_pretty(&data).unwrap_or_default()).is_err() {
                self.message = Some((true, "Não foi possível salvar a conta local.".into()));
                return;
            }
        }
        self.session = Some(AuthSession {
            access_token: Self::digest("session", &email, &self.password),
            refresh_token: String::new(),
            user_id: Self::digest("user", &email, ""),
            email,
            expires_at: client::unix_seconds() + 60 * 60 * 24 * 365,
        });
        self.password.clear();
        self.confirmation.clear();
        self.message = None;
        self.screen = Screen::SignIn;
        ctx.request_repaint();
    }

    pub(crate) fn unlocked(&self) -> bool {
        self.session.as_ref().is_some_and(|session| session.expires_at > client::unix_seconds())
    }

    pub(crate) fn has_session(&self) -> bool {
        self.session.is_some()
    }

    pub(crate) fn account_email(&self) -> Option<&str> {
        self.session.as_ref().filter(|_| self.unlocked()).map(|session| session.email.as_str())
    }

    fn navigate(&mut self, screen: Screen) {
        // Dropping the receiver makes late results harmless, including completed recovery.
        self.pending = None;
        let recovery = self.recovery.take();
        if !self.local_only
            && let (Ok(config), Some(session)) = (&self.config, recovery)
        {
            client::send(config, Action::SignOut { access_token: session.access_token }, |_| {});
        }
        self.password.clear();
        self.confirmation.clear();
        self.code.clear();
        self.reveal = false;
        self.message = None;
        self.screen = screen;
    }

    fn start(&mut self, operation: Operation, action: Action, ctx: &Context) {
        if self.local_only || self.pending.is_some() {
            return;
        }
        let config = match &self.config {
            Ok(config) => config,
            Err(error) => {
                self.message = Some((true, error.clone()));
                return;
            }
        };
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(Pending { operation, started: web_time::Instant::now(), receiver });
        self.message = None;
        let ctx = ctx.clone();
        client::send(config, action, move |result| {
            let _ = sender.send(result);
            ctx.request_repaint();
        });
    }

    /// Called only by an engine UI request. Credentials are read here, not command parameters.
    pub(crate) fn dispatch(&mut self, command: &str, ctx: Option<&Context>) {
        let Some(ctx) = ctx else { return };
        if command == "auth.signOut" {
            self.sign_out(ctx);
            return;
        }
        if self.pending.is_some() || self.unlocked() {
            return;
        }
        if self.local_only {
            match (command, self.screen) {
                ("auth.signIn", Screen::SignIn) => self.local_auth(false, ctx),
                ("auth.signUp", Screen::SignUp) => self.local_auth(true, ctx),
                ("auth.sendRecovery", _) | ("auth.verifyRecovery", _) | ("auth.updatePassword", _) | ("auth.verifySignup", _) => {
                    self.message =
                        Some((true, "Esta conta é local. A recuperação de senha ainda não foi configurada; nenhum e-mail será enviado.".into()));
                    ctx.request_repaint();
                }
                _ => {}
            }
            return;
        }
        if !self.email_codes_enabled && command == "auth.verifySignup" {
            self.message = Some((true, "Confirme seu e-mail pelo link recebido e depois entre com sua senha.".into()));
            return;
        }
        let action = match (command, self.screen) {
            ("auth.signIn", Screen::SignIn) => {
                Some((Operation::SignIn, Action::SignIn { email: self.email.clone(), password: self.password.clone() }))
            }
            ("auth.signUp", Screen::SignUp) => {
                if self.password != self.confirmation {
                    self.message = Some((true, "As senhas precisam ser iguais.".into()));
                    return;
                }
                Some((Operation::SignUp, Action::SignUp { email: self.email.clone(), password: self.password.clone() }))
            }
            ("auth.sendRecovery", Screen::Recovery) => Some((Operation::SendRecovery, Action::SendRecovery { email: self.email.clone() })),
            ("auth.verifySignup", Screen::VerifySignup) => {
                Some((Operation::VerifySignup, Action::VerifySignup { email: self.email.clone(), token: self.code.trim().to_owned() }))
            }
            ("auth.verifyRecovery", Screen::VerifyRecovery) => {
                let action = if self.email_codes_enabled {
                    Action::VerifyRecovery { email: self.email.clone(), token: self.code.trim().to_owned() }
                } else {
                    Action::VerifyRecoveryLink { link: self.code.trim().to_owned() }
                };
                Some((Operation::VerifyRecovery, action))
            }
            ("auth.updatePassword", Screen::NewPassword) => {
                if self.password != self.confirmation {
                    self.message = Some((true, "As senhas precisam ser iguais.".into()));
                    return;
                }
                self.recovery.as_ref().filter(|session| session.expires_at > client::unix_seconds()).map(|session| {
                    (
                        Operation::UpdatePassword,
                        Action::UpdatePassword { access_token: session.access_token.clone(), password: self.password.clone() },
                    )
                })
            }
            _ => None,
        };
        if let Some((operation, action)) = action {
            self.start(operation, action, ctx);
        } else if command == "auth.updatePassword" && self.screen == Screen::NewPassword {
            self.navigate(Screen::Recovery);
            self.message = Some((true, "O código expirou. Solicite um novo para continuar.".into()));
        }
    }

    fn sign_out(&mut self, ctx: &Context) {
        let session = self.session.take().or_else(|| self.recovery.take());
        self.navigate(Screen::SignIn);
        if !self.local_only
            && let (Ok(config), Some(session)) = (&self.config, session)
        {
            // Lock locally immediately. Server-side revocation is best effort when offline.
            client::send(config, Action::SignOut { access_token: session.access_token }, |_| {});
        }
        ctx.request_repaint();
    }

    pub(crate) fn poll(&mut self, ctx: &Context) {
        let response = self.pending.as_ref().and_then(|pending| match pending.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Disconnected) => Some(Err("A conexão foi interrompida. Tente novamente.".into())),
            Err(TryRecvError::Empty) if pending.started.elapsed() >= Duration::from_secs(35) => {
                Some(Err("O serviço demorou para responder. Verifique sua conexão e tente novamente.".into()))
            }
            Err(TryRecvError::Empty) => None,
        });
        if let Some(response) = response
            && let Some(pending) = self.pending.take()
        {
            self.complete(pending.operation, response);
            ctx.request_repaint();
        }
        let now = client::unix_seconds();
        if let Some(session) = &self.session {
            if session.expires_at <= now {
                self.session = None;
                self.navigate(Screen::SignIn);
                self.message = Some((true, "Sua sessão expirou. Entre novamente para continuar.".into()));
                ctx.request_repaint();
            } else if !self.local_only && session.expires_at.saturating_sub(now) <= 60 && self.pending.is_none() {
                let action = Action::Refresh { refresh_token: session.refresh_token.clone() };
                self.start(Operation::Refresh, action, ctx);
            }
            ctx.request_repaint_after(Duration::from_secs(15));
        }
        if self.pending.is_some() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    fn complete(&mut self, operation: Operation, result: Result<Outcome, String>) {
        match (operation, result) {
            (Operation::VerifyRecovery, Ok(Outcome::Session(session))) => {
                self.recovery = Some(session);
                self.screen = Screen::NewPassword;
                self.code.clear();
                self.message = None;
            }
            (Operation::SignIn | Operation::SignUp | Operation::VerifySignup | Operation::Refresh, Ok(Outcome::Session(session))) => {
                self.session = Some(session);
                self.password.clear();
                self.confirmation.clear();
                self.code.clear();
                self.message = None;
            }
            (Operation::SignUp, Ok(Outcome::Message(message))) => {
                self.navigate(Screen::VerifySignup);
                self.message = Some((false, message));
            }
            (Operation::SendRecovery, Ok(Outcome::Message(message))) => {
                let message = if self.email_codes_enabled {
                    message
                } else {
                    "Se houver uma conta com esse e-mail, você receberá uma mensagem de recuperação. Copie o link da mensagem sem abri-lo.".into()
                };
                self.navigate(Screen::VerifyRecovery);
                self.message = Some((false, message));
            }
            (Operation::UpdatePassword, Ok(Outcome::Message(message))) => {
                self.navigate(Screen::SignIn);
                self.message = Some((false, message));
            }
            (operation, Err(message)) => {
                if operation == Operation::Refresh {
                    self.session = None;
                    self.navigate(Screen::SignIn);
                }
                self.message = Some((true, message));
            }
            _ => {
                self.message = Some((true, "Não foi possível concluir esta etapa. Tente novamente.".into()));
            }
        }
    }
}

impl crate::FigApp {
    /// Enable the account entry in the shipped desktop and browser apps. Embedders/CLI keep
    /// using the engine independently. Public config may be overridden at build time.
    pub fn enable_designflow_login(&mut self) {
        // Keep the operating system's native interface scale. The workspace preferences
        // restore an explicit accessibility choice independently of the canvas zoom.
        if self.session.prefs.theme == "system" {
            self.session.prefs.theme = "light".into();
        }
        if self.session.prefs.language == "system" {
            self.session.prefs.language = "pt-BR".into();
        }
        if self.ui.right_width == crate::PANEL_DEFAULT || self.ui.right_width == 280.0 {
            let left = if self.ui.left_width == crate::PANEL_DEFAULT { 224.0 } else { self.ui.left_width };
            self.set_panel_widths(left, 288.0);
        }
        let defaults: serde_json::Value = serde_json::from_str(include_str!("auth/config.json")).unwrap_or_default();
        let url = option_env!("DESIGNFLOW_SUPABASE_URL").or_else(|| defaults.get("url").and_then(serde_json::Value::as_str)).unwrap_or_default();
        let key = option_env!("DESIGNFLOW_SUPABASE_PUBLISHABLE_KEY")
            .or_else(|| defaults.get("publishable_key").and_then(serde_json::Value::as_str))
            .unwrap_or_default();
        let mut auth = Auth::new(Config::new(url, key));
        auth.email_codes_enabled = defaults.get("email_codes_enabled").and_then(serde_json::Value::as_bool).unwrap_or(false);
        self.auth = Some(auth);
    }
}

/// True means the account screen owns this frame. It runs before editor shortcuts and panels.
pub(crate) fn gate(app: &mut crate::FigApp, ui: &mut egui::Ui) -> bool {
    if app.auth.as_ref().is_some_and(|auth| !auth.unlocked()) {
        app.poll_dialog(ui.ctx());
        crate::control::drain(app, ui.ctx());
    }
    let Some(auth) = &mut app.auth else { return false };
    if auth.unlocked() {
        return false;
    }
    crate::tabs::login_title_bar(app, ui);
    let command = app.auth.as_mut().and_then(|auth| view::show(auth, ui));
    if let Some(command) = command {
        app.run(command, serde_json::json!({}));
    }
    crate::windows::confirm_close(app, ui.ctx());
    true
}
