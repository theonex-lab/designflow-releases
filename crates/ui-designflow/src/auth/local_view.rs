//! The supplied DesignFlow login composition, scaled in logical points for every display density.
use super::{Auth, Screen};
use crate::theme::designflow_regular as regular;
use egui::{Align2, Color32, Context, Id, Margin, Rect, RichText, Sense, Stroke, StrokeKind, TextureHandle, Ui, pos2, vec2};

const INK: Color32 = Color32::from_rgb(40, 41, 59);
const MUTED: Color32 = Color32::from_rgb(111, 113, 132);
const PURPLE: Color32 = Color32::from_rgb(97, 0, 255);
pub(super) const COMPOSITION_SCALE: f32 = 0.72;

fn texture(ctx: &Context, name: &'static str, bytes: &[u8]) -> Option<TextureHandle> {
    let id = Id::unique(name);
    if let Some(handle) = ctx.data(|data| data.get_temp::<TextureHandle>(id)) {
        return Some(handle);
    }
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let pixels = egui::ColorImage::from_rgba_unmultiplied([image.width() as usize, image.height() as usize], image.as_raw());
    let handle = ctx.load_texture(name, pixels, egui::TextureOptions::LINEAR);
    ctx.data_mut(|data| data.insert_temp(id, handle.clone()));
    Some(handle)
}

pub(super) fn show(auth: &mut Auth, root: &mut Ui) -> Option<&'static str> {
    if matches!(auth.screen, Screen::SignIn | Screen::SignUp) {
        return sign_in(auth, root);
    }
    let mut command = None;
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(Color32::from_rgb(250, 248, 255))).show(root, |ui| {
        ui.style_mut().visuals = egui::Visuals::light();
        ui.style_mut().visuals.override_text_color = Some(INK);
        ui.style_mut().visuals.selection.bg_fill = Color32::from_rgb(229, 207, 255);
        let screen = ui.max_rect();
        if let Some(background) =
            texture(ui.ctx(), "designflow-login-reference-background", include_bytes!("../../../../assets/designflow/login-background.png"))
        {
            ui.painter().image(background.id(), screen, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        // Keep the form compact with generous breathing room on a Full HD desktop.
        let s = (screen.width() / 1672.0).min(screen.height() / 941.0) * COMPOSITION_SCALE;
        let origin = screen.center() - vec2(836.0, 470.5) * s;
        let area = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(origin + vec2(x, y) * s, vec2(w, h) * s);
        if let Some(logo) =
            texture(ui.ctx(), "designflow-login-original-hd-logo", include_bytes!("../../../../assets/designflow/logo-transparent.png"))
        {
            ui.painter().image(
                logo.id(),
                area(550.0, 103.0, 572.0, 106.0),
                Rect::from_min_max(pos2(0.0258, 0.2652), pos2(0.9908, 0.8025)),
                Color32::WHITE,
            );
        }
        let signup = auth.screen == Screen::SignUp;
        let recovery = !matches!(auth.screen, Screen::SignIn | Screen::SignUp);
        let title = if signup {
            "Crie sua conta"
        } else if recovery {
            "Recupere seu acesso"
        } else {
            "Acesse sua conta"
        };
        ui.painter().text(area(550.0, 241.0, 572.0, 45.0).center(), Align2::CENTER_CENTER, title, regular(32.0 * s), INK);
        let mut navigate = None;
        if recovery {
            let text =
                "Sua conta está salva apenas neste computador.\nA recuperação de senha ainda não foi configurada.\nNenhum e-mail será enviado.";
            ui.scope_builder(egui::UiBuilder::new().max_rect(area(550.0, 328.0, 572.0, 130.0)), |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(text).font(regular(24.0 * s)).color(INK));
                });
            });
            if button(ui, area(550.0, 518.0, 572.0, 72.0), "Voltar para entrar", s, true) {
                navigate = Some(Screen::SignIn);
            }
        } else {
            let mut enter = input(ui, area(550.0, 308.0, 572.0, 70.0), "E-mail", &mut auth.email, None, FieldIcon::Mail, s);
            enter |= input(ui, area(550.0, 396.0, 572.0, 70.0), "Senha", &mut auth.password, Some(&mut auth.reveal), FieldIcon::Lock, s);
            if signup {
                enter |=
                    input(ui, area(550.0, 484.0, 572.0, 70.0), "Confirmar senha", &mut auth.confirmation, Some(&mut auth.reveal), FieldIcon::Lock, s);
                if button(ui, area(550.0, 608.0, 572.0, 72.0), "Criar conta", s, true) || enter {
                    command = Some("auth.signUp");
                }
                if link(ui, area(550.0, 734.0, 572.0, 45.0), "Já tem uma conta? Entrar", s, PURPLE) {
                    navigate = Some(Screen::SignIn);
                }
            } else {
                if link(ui, area(905.0, 480.0, 217.0, 42.0), "Esqueceu sua senha?", s, INK) {
                    navigate = Some(Screen::Recovery);
                }
                if button(ui, area(550.0, 546.0, 572.0, 72.0), "Entrar", s, true) || enter {
                    command = Some("auth.signIn");
                }
                let y = origin.y + 663.0 * s;
                ui.painter()
                    .line_segment([pos2(origin.x + 550.0 * s, y), pos2(origin.x + 786.0 * s, y)], Stroke::new(s, Color32::from_rgb(188, 188, 205)));
                ui.painter()
                    .line_segment([pos2(origin.x + 885.0 * s, y), pos2(origin.x + 1122.0 * s, y)], Stroke::new(s, Color32::from_rgb(188, 188, 205)));
                ui.painter().text(origin + vec2(836.0, 663.0) * s, Align2::CENTER_CENTER, "ou", regular(25.0 * s), MUTED);
                // The chosen local-account mode has no external identity provider.
                if button(ui, area(550.0, 706.0, 572.0, 71.0), "Criar conta", s, false) {
                    navigate = Some(Screen::SignUp);
                }
            }
            ui.painter().text(
                area(550.0, 813.0, 572.0, 40.0).center(),
                Align2::CENTER_CENTER,
                "Sua conta fica salva neste computador",
                regular(22.0 * s),
                MUTED,
            );
            if let Some((error, message)) = &auth.message {
                let y = if signup { 561.0 } else { 620.0 };
                ui.scope_builder(egui::UiBuilder::new().max_rect(area(550.0, y, 572.0, 42.0)), |ui| {
                    ui.label(RichText::new(message).font(regular(17.0 * s)).color(if *error { Color32::from_rgb(175, 35, 71) } else { MUTED }));
                });
            }
        }
        if let Some(target) = navigate {
            auth.navigate(target);
            ui.ctx().request_repaint();
            command = None;
        }
    });
    command
}

fn sign_in(auth: &mut Auth, root: &mut Ui) -> Option<&'static str> {
    let mut command = None;
    let signup = auth.screen == Screen::SignUp;
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(Color32::WHITE)).show(root, |ui| {
        ui.style_mut().visuals = egui::Visuals::light();
        let screen = ui.max_rect();
        // At Full HD the form is about 430 points wide, with 45-point fields.
        // Anchor the visible content, rather than the old reference canvas, at the center.
        let s = (screen.width() / 700.0).min(screen.height() / 850.0).min(0.76);
        let center = screen.center().x;
        let content_midpoint = if signup { 521.5 } else { 608.0 };
        let top = screen.center().y - content_midpoint * s;
        let area = |x: f32, y: f32, w: f32, h: f32| {
            Rect::from_min_size(pos2(center + (x - 995.0) * s, top + y * s), vec2(w, h) * s)
        };
        let paint = |ui: &Ui, r: Rect, label: &str, size: f32, color, bold| {
            ui.painter().text(r.center(), Align2::CENTER_CENTER, label,
                if bold { arial(size * s, true) } else if size == 15.0 && matches!(label, "Email" | "Senha" | "Confirmar senha") { arial(size * s, false) } else { regular(size * s) }, color);
        };
        asset(ui, Rect::from_min_size(screen.min + vec2(42.0, 30.0) * s, vec2(281.0, 49.0) * s),
            "signin-logo", include_bytes!("../../../../assets/designflow/signin-logo.png"));
        paint(ui, area(656.5, 225.0, 677.0, 74.0), if signup { "Crie sua conta" } else { "Bem-vindo de volta" }, 56.0, Color32::BLACK, true);
        paint(ui, area(787.0, 317.0, 416.0, 34.0), if signup { "Comece a criar no DesignFlow" } else { "Faça login para continuar criando" }, 22.0, Color32::from_gray(105), false);
        paint(ui, area(710.0, 370.0, 46.0, 22.0), "Email", 15.0, Color32::BLACK, false);
        paint(ui, area(712.0, 464.0, 50.0, 22.0), "Senha", 15.0, Color32::BLACK, false);
        let enter = input(ui, area(713.0, 395.0, 563.0, 59.0), "Insira seu email", &mut auth.email, None, FieldIcon::Mail, s)
            | input(ui, area(713.0, 489.0, 563.0, 59.0), "Insira sua senha", &mut auth.password, Some(&mut auth.reveal), FieldIcon::Lock, s);
        if signup {
            paint(ui, area(710.0, 558.0, 132.0, 22.0), "Confirmar senha", 15.0, Color32::BLACK, false);
            let confirm = input(ui, area(713.0, 583.0, 563.0, 59.0), "Repita sua senha", &mut auth.confirmation, Some(&mut auth.reveal), FieldIcon::Lock, s);
            if let Some((error, message)) = &auth.message {
                paint(ui, area(713.0, 648.0, 563.0, 32.0), message, 15.0, if *error { Color32::from_rgb(175, 35, 71) } else { MUTED }, false);
            }
            if button(ui, area(711.0, 691.0, 568.0, 62.0), "Criar conta", s, true) || enter || confirm {
                command = Some("auth.signUp");
            }
            if link(ui, area(899.0, 783.0, 194.0, 35.0), "Voltar para o login", s, PURPLE) {
                auth.navigate(Screen::SignIn);
                ui.ctx().request_repaint();
                command = None;
            }
            return;
        }
        if link(ui, area(1120.0, 562.0, 160.0, 33.0), "Esqueceu sua senha?", s, PURPLE) {
            auth.navigate(Screen::Recovery);
        }
        if let Some((_, message)) = &auth.message {
            paint(ui, area(713.0, 591.0, 563.0, 30.0), message, 15.0, Color32::from_rgb(175, 35, 71), false);
        }
        if button(ui, area(711.0, 623.0, 568.0, 62.0), "Fazer login", s, true) || enter {
            command = Some("auth.signIn");
        }
        for x in [723.0, 1023.0] {
            let r = area(x, 727.0, 244.0, 0.0);
            ui.painter().line_segment([r.left_top(), r.right_top()], Stroke::new(s, Color32::from_gray(220)));
        }
        paint(ui, area(967.0, 711.0, 56.0, 32.0), "Ou", 20.0, Color32::BLACK, false);
        for (y, label, name, bytes, size) in [
            (754.0, "Continuar com Google", "signin-google", include_bytes!("../../../../assets/designflow/signin-google.png").as_slice(), vec2(55.0, 37.0)),
            (828.0, "Continuar com Apple", "signin-apple", include_bytes!("../../../../assets/designflow/signin-apple.png").as_slice(), vec2(60.0, 51.0)),
        ] {
            let rect = area(713.5, y, 563.0, 59.0);
            ui.painter().rect_stroke(rect, (10.0 * s) as u8, Stroke::new(s, Color32::from_gray(220)), StrokeKind::Inside);
            asset(ui, Rect::from_center_size(area(880.0, y + 30.0, 0.0, 0.0).center(), size * s), name, bytes);
            paint(ui, area(915.0, y, 230.0, 59.0), label, 21.0, Color32::BLACK, false);
            ui.interact(rect, ui.scope_id().with(name), Sense::hover())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Indisponível no modo de conta local");
        }
        paint(ui, area(872.0, 911.0, 151.0, 32.0), "Não tem uma conta?", 15.0, MUTED, false);
        if link(ui, area(1027.0, 911.0, 91.0, 32.0), "Cadastre-se", s, PURPLE) {
            auth.navigate(Screen::SignUp);
            ui.ctx().request_repaint();
        }
        paint(ui, area(923.5, 962.0, 143.0, 29.0), "Privacidade     |     Termos", 12.0, MUTED, false);
    });
    command
}

fn arial(size: f32, bold: bool) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name(if bold { "login-arial-bold".into() } else { "login-arial".into() }))
}

fn asset(ui: &Ui, rect: Rect, name: &'static str, bytes: &[u8]) {
    if let Some(image) = texture(ui.ctx(), name, bytes) {
        ui.painter().image(image.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
}

#[derive(Clone, Copy)]
enum FieldIcon {
    Mail,
    Lock,
}

fn input(ui: &mut Ui, rect: Rect, label: &str, value: &mut String, mut reveal: Option<&mut bool>, icon: FieldIcon, s: f32) -> bool {
    ui.painter().rect_filled(rect, (10.0 * s) as u8, Color32::WHITE);
    ui.painter().rect_stroke(rect, (10.0 * s) as u8, Stroke::new(s, Color32::from_gray(220)), StrokeKind::Inside);
    let icon_box = Rect::from_center_size(pos2(rect.left() + 32.0 * s, rect.center().y), vec2(32.0, 32.0) * s);
    field_icon(ui, icon_box, icon, s);
    let secret = reveal.is_some();
    let password = reveal.as_deref().is_some_and(|visible| !*visible);
    let edit_rect = Rect::from_min_max(
        pos2(rect.left() + 61.0 * s, rect.center().y - 15.0 * s),
        pos2(rect.right() - if secret { 68.0 * s } else { 22.0 * s }, rect.center().y + 21.0 * s),
    );
    let response = ui.put(
        edit_rect,
        egui::TextEdit::singleline(value)
            .id_salt(("designflow-account", label))
            .hint_text(RichText::new(label).font(regular(20.0 * s)).color(MUTED))
            .password(password)
            .char_limit(if secret { 1024 } else { 254 })
            .font(regular(20.0 * s))
            .margin(Margin::ZERO)
            .frame(egui::Frame::NONE),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, ui.is_enabled(), label));
    if let Some(visible) = &mut reveal {
        let eye_box = Rect::from_center_size(pos2(rect.right() - 29.0 * s, rect.center().y), vec2(38.0, 38.0) * s);
        let eye = ui.interact(eye_box, response.id.with("visibility"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        crate::a11y::button(&eye, if **visible { "Ocultar senha" } else { "Mostrar senha" });
        eye_icon(ui, eye_box.shrink(3.0 * s), s, **visible);
        if eye.clicked() {
            **visible = !**visible;
            response.request_focus();
        }
    }
    if response.has_focus() {
        ui.painter().rect_stroke(rect, (10.0 * s) as u8, Stroke::new(1.5 * s, PURPLE), StrokeKind::Inside);
    }
    response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}

fn field_icon(ui: &Ui, r: Rect, icon: FieldIcon, s: f32) {
    let (name, bytes, size): (&'static str, &[u8], _) = match icon {
        FieldIcon::Mail => ("signin-email", include_bytes!("../../../../assets/designflow/signin-email.png"), vec2(51.0, 45.0)),
        FieldIcon::Lock => ("signin-lock", include_bytes!("../../../../assets/designflow/signin-lock.png"), vec2(51.0, 45.0)),
    };
    if let Some(image) = texture(ui.ctx(), name, bytes) {
        ui.painter().image(
            image.id(),
            Rect::from_center_size(r.center(), size * s),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

fn eye_icon(ui: &Ui, r: Rect, s: f32, revealed: bool) {
    let stroke = Stroke::new(2.5 * s, Color32::from_rgb(61, 65, 82));
    if let Some(image) = texture(ui.ctx(), "signin-eye", include_bytes!("../../../../assets/designflow/signin-eye.png")) {
        ui.painter().image(
            image.id(),
            Rect::from_center_size(r.center(), vec2(52.0, 28.0) * s),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    if revealed {
        ui.painter().line_segment([r.left_bottom(), r.right_top()], stroke);
    }
}

fn button(ui: &mut Ui, rect: Rect, label: &str, s: f32, primary: bool) -> bool {
    let response = ui.interact(rect, ui.scope_id().with(("designflow-login-button", label)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    crate::a11y::button(&response, label);
    let fill = if primary {
        if response.hovered() { Color32::from_rgb(124, 0, 234) } else { PURPLE }
    } else if response.hovered() {
        Color32::from_white_alpha(155)
    } else {
        Color32::from_white_alpha(60)
    };
    ui.painter().rect_filled(rect, (10.0 * s) as u8, fill);
    if !primary {
        ui.painter().rect_stroke(rect, (24.0 * s) as u8, Stroke::new(1.4 * s, Color32::from_rgb(181, 180, 201)), StrokeKind::Inside);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        arial(20.0 * s, false),
        if primary { Color32::WHITE } else { INK },
    );
    if response.has_focus() {
        ui.painter().rect_stroke(rect.expand(3.0 * s), (27.0 * s) as u8, Stroke::new(2.0 * s, PURPLE), StrokeKind::Outside);
    }
    response.clicked()
}

fn link(ui: &mut Ui, rect: Rect, label: &str, s: f32, color: Color32) -> bool {
    let response = ui.interact(rect, ui.scope_id().with(("designflow-login-link", label)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    crate::a11y::button(&response, label);
    let font = if label == "Esqueceu sua senha?" { arial(15.0 * s, false) } else { regular(15.0 * s) };
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, font, if response.hovered() { PURPLE } else { color });
    response.clicked()
}
