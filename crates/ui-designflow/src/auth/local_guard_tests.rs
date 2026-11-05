use super::*;

fn local_auth() -> Auth {
    Auth::new(Err("No remote account service in this test".into()))
}

fn session() -> AuthSession {
    AuthSession {
        access_token: "synthetic-local-session".into(),
        refresh_token: String::new(),
        user_id: "synthetic-local-user".into(),
        email: "person@example.org".into(),
        expires_at: client::unix_seconds() + 30,
    }
}

#[test]
fn mismatched_signup_password_is_rejected_before_accessing_account_file() {
    let mut auth = local_auth();
    auth.password = "synthetic-password".into();
    auth.confirmation = "different-password".into();
    auth.local_auth(true, &Context::default());
    assert!(!auth.unlocked());
    assert_eq!(auth.message.as_ref().map(|(_, text)| text.as_str()), Some("As senhas precisam ser iguais."));
}

#[test]
fn local_recovery_never_starts_remote_requests_or_unlocks_editor() {
    let mut auth = local_auth();
    auth.screen = Screen::Recovery;
    let ctx = Context::default();
    for command in ["auth.sendRecovery", "auth.verifyRecovery", "auth.updatePassword", "auth.verifySignup"] {
        auth.dispatch(command, Some(&ctx));
        assert!(auth.pending.is_none());
        assert!(!auth.unlocked());
        assert!(auth.message.as_ref().unwrap().1.contains("nenhum e-mail será enviado"));
    }
}

#[test]
fn local_session_does_not_request_remote_refresh_and_logout_clears_it() {
    let mut auth = local_auth();
    let ctx = Context::default();
    auth.session = Some(session());
    auth.poll(&ctx);
    assert!(auth.unlocked());
    assert!(auth.pending.is_none());
    auth.sign_out(&ctx);
    assert!(!auth.unlocked());
    assert!(auth.pending.is_none());
    assert!(auth.recovery.is_none());
}

#[test]
fn local_login_create_account_and_back_links_keep_editor_locked() {
    let ctx = Context::default();
    crate::theme::install_fonts(&ctx, false, 0);
    let mut auth = local_auth();
    let size = egui::vec2(1920.0, 1080.0);
    let scale = (size.x / 1672.0).min(size.y / 941.0) * local_view::COMPOSITION_SCALE;
    let origin = egui::pos2(size.x * 0.5, size.y * 0.5) - egui::vec2(836.0, 470.5) * scale;
    let frame = |auth: &mut Auth, events: Vec<egui::Event>| {
        let _ =
            ctx.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)), events, ..Default::default() }, |ui| {
                assert!(super::local_view::show(auth, ui).is_none());
            });
    };
    for _ in 0..3 {
        frame(&mut auth, vec![]);
    }
    // The secondary action on SignIn occupies (550,706) to (1122,777)
    // in reference coordinates; clicks use the same layout transform as the form.
    for point in [egui::vec2(836.0, 741.5), egui::vec2(836.0, 756.5)] {
        let position = origin + point * scale;
        for pressed in [true, false] {
            frame(
                &mut auth,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton { pos: position, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE },
                ],
            );
        }
        if point.y == 741.5 {
            assert!(auth.screen == Screen::SignUp);
        } else {
            assert!(auth.screen == Screen::SignIn);
        }
        assert!(!auth.unlocked());
        assert!(auth.pending.is_none());
        frame(&mut auth, vec![]);
    }
}
