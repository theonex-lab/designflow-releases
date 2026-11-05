//! DesignFlow's personal-space account screen, shared by desktop and browser.
use super::{Auth, Operation, Screen};
use crate::icons::{self, Icon};
use crate::theme::{Tokens, designflow_regular as regular, designflow_semibold as semibold};
use egui::{Align, Align2, Color32, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind, TextEdit, Ui, pos2, vec2};

#[derive(Clone, Copy)]
struct Metrics {
    scale: f32,
    width: f32,
    body: f32,
    field: f32,
}
impl Metrics {
    fn new(size: egui::Vec2) -> Self {
        let scale = (size.y / 984.0).clamp(0.8, 1.0).min(if size.x < 480.0 { 0.88 } else { 1.0 });
        let width = (size.x - 32.0).clamp(120.0, 614.0 * scale);
        Self { scale, width, body: (20.0 * scale).max(16.0), field: (56.0 * scale).max(48.0) }
    }
    fn gap(self, ui: &mut Ui, points: f32) {
        ui.add_space(points * self.scale);
    }
}

pub(super) fn show(auth: &mut Auth, root: &mut Ui) -> Option<&'static str> {
    if auth.local_only {
        return super::local_view::show(auth, root);
    }
    let t = Tokens::designflow_login();
    let mut command = None;
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.bg)).show(root, |ui| {
        ui.style_mut().visuals = egui::Visuals::light();
        ui.style_mut().visuals.override_text_color = Some(t.text);
        ui.style_mut().visuals.selection.bg_fill = t.selected_row;
        ui.style_mut().visuals.hyperlink_color = t.accent;
        ui.style_mut().spacing.item_spacing = vec2(8.0, 0.0);
        let available = ui.available_size();
        let m = Metrics::new(available);
        decorative_background(ui, &t, available);
        let height_id = ui.scope_id().with(("designflow_account_height", auth.screen as u8, (m.scale * 100.0) as u32));
        let estimated = if auth.screen == Screen::SignUp { 856.0 } else { 780.0 } * m.scale;
        let height = ui.ctx().data(|data| data.get_temp::<f32>(height_id)).unwrap_or(estimated);
        let top = ((available.y - height) * 0.5 + 36.0 * m.scale).max(24.0);
        egui::ScrollArea::vertical().id_salt("designflow_login_scroll").show(ui, |ui| {
            ui.add_space(top);
            let group = ui.with_layout(Layout::top_down(Align::Center), |ui| {
                logo(auth, ui, &t, m);
                m.gap(ui, 34.0);
                egui::Frame::new().fill(Color32::TRANSPARENT).stroke(Stroke::NONE).inner_margin(Margin::same(0)).show(ui, |ui| {
                    // Keep the form comfortably separated from the screen edges on a
                    // 1920x1080 display while remaining usable on smaller windows.
                    ui.set_width((m.width - 48.0).clamp(420.0, 620.0));
                    ui.with_layout(Layout::top_down(Align::Min), |ui| command = form(auth, ui, &t, m));
                });
                m.gap(ui, 28.0);
                ui.label(RichText::new("Seu espaço para criar").font(regular(m.body)).color(t.text_secondary));
            });
            let measured = group.response.rect.height();
            ui.ctx().data_mut(|data| data.insert_temp(height_id, measured));
            if (measured - height).abs() > 1.0 {
                ui.ctx().request_repaint();
            }
            ui.add_space(24.0);
        });
    });
    command
}

fn decorative_background(ui: &mut Ui, t: &Tokens, size: egui::Vec2) {
    let painter = ui.painter();
    let w = size.x;
    let h = size.y;
    let lavender = Color32::from_rgba_premultiplied(185, 126, 255, 34);
    let soft = Color32::from_rgba_premultiplied(135, 61, 255, 70);
    let line = Stroke::new(1.4, Color32::from_rgba_premultiplied(126, 48, 255, 150));

    // Large, quiet corner shapes keep the 1920x1080 composition airy without competing with
    // the form. They are drawn procedurally so they stay sharp at any display scale.
    painter.circle_stroke(pos2(-70.0, -80.0), 330.0, line);
    painter.circle_stroke(pos2(w + 70.0, h + 80.0), 360.0, line);
    painter.circle_filled(pos2(w + 122.0, 36.0), 210.0, lavender);
    painter.circle_filled(pos2(-90.0, h + 40.0), 220.0, lavender);
    painter.line_segment([pos2(w - 310.0, 0.0), pos2(w - 430.0, 205.0)], Stroke::new(1.0, soft));
    painter.line_segment([pos2(0.0, h - 250.0), pos2(270.0, h)], Stroke::new(1.0, soft));

    for row in 0..3 {
        for col in 0..6 {
            painter.circle_filled(pos2(78.0 + col as f32 * 27.0, 46.0 + row as f32 * 24.0), 2.6, t.accent.linear_multiply(0.55));
        }
    }
    for row in 0..3 {
        for col in 0..4 {
            painter.circle_filled(pos2(w - 150.0 + col as f32 * 27.0, h - 92.0 + row as f32 * 24.0), 2.6, t.accent.linear_multiply(0.55));
        }
    }
    painter.text(pos2(315.0, h * 0.34), egui::Align2::CENTER_CENTER, "+", semibold(32.0), t.accent);
    painter.text(pos2(w - 250.0, h * 0.72), egui::Align2::CENTER_CENTER, "+", semibold(32.0), t.accent);
}

fn logo(auth: &mut Auth, ui: &mut Ui, t: &Tokens, m: Metrics) {
    if auth.logo.is_none()
        && let Ok(image) = image::load_from_memory(include_bytes!("../../../../assets/designflow/logo-transparent.png"))
    {
        let max_side = ui.ctx().input(|input| input.max_texture_side).clamp(1, 2048) as u32;
        let image = if image.width() > max_side || image.height() > max_side {
            image.resize(max_side, max_side, image::imageops::FilterType::Lanczos3)
        } else {
            image
        };
        let rgba = image.to_rgba8();
        let image = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
        auth.logo = Some(ui.ctx().load_texture("designflow_original_logo_transparent", image, egui::TextureOptions::LINEAR));
    }
    let width = (478.0 * m.scale).min((m.width - 36.0).max(100.0));
    if let Some(logo) = &auth.logo {
        let uv = Rect::from_min_max(pos2(0.018, 0.24), pos2(0.999, 0.83));
        ui.add(egui::Image::new((logo.id(), vec2(width, width / 4.99))).uv(uv).alt_text("DesignFlow"));
    } else {
        ui.label(RichText::new("DesignFlow").font(semibold(36.0 * m.scale)).color(t.accent));
    }
}

fn form(auth: &mut Auth, ui: &mut Ui, t: &Tokens, m: Metrics) -> Option<&'static str> {
    let tabbed = matches!(auth.screen, Screen::SignIn | Screen::SignUp);
    let (title, subtitle, label, command) = copy(auth);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(title).font(semibold(38.0 * m.scale)).color(t.text));
        m.gap(ui, 8.0);
        ui.label(RichText::new(subtitle).font(regular(m.body)).color(t.text_secondary));
    });
    m.gap(ui, 34.0);
    let mut navigate = if tabbed { account_tabs(auth, ui, t, m) } else { None };
    if tabbed {
        m.gap(ui, 28.0);
    }
    if auth.screen == Screen::VerifySignup && !auth.email_codes_enabled {
        ui.label(
            RichText::new("Abra o link de confirmação do e-mail. Depois, volte aqui para entrar com sua senha.")
                .font(regular(m.body))
                .color(t.text_secondary),
        );
        m.gap(ui, 28.0);
        if primary_button(ui, "Já confirmei meu e-mail", true, t, m) {
            navigate = Some(Screen::SignIn);
        }
        back_link(auth, ui, t, m, &mut navigate);
        return finish_navigation(auth, ui, navigate);
    }
    if auth.config.is_err() && !auth.local_only {
        notice(ui, t, m, true, "O acesso ainda está sendo preparado. A configuração do serviço de contas precisa ser concluída.");
        m.gap(ui, 18.0);
    }
    let enabled = auth.pending.is_none() && (auth.local_only || auth.config.is_ok());
    let mut enter = false;
    ui.add_enabled_ui(enabled, |ui| enter = fields(auth, ui, t, m));
    if auth.screen == Screen::SignIn {
        m.gap(ui, 12.0);
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if text_link(ui, "Esqueci minha senha", auth.pending.is_none(), t, m) {
                    navigate = Some(Screen::Recovery);
                }
            });
        });
    }
    if let Some((error, message)) = &auth.message {
        m.gap(ui, 16.0);
        notice(ui, t, m, *error, message);
    }
    m.gap(ui, 28.0);
    let button_text = match auth.pending.as_ref().map(|pending| pending.operation) {
        Some(Operation::SignIn) => "Entrando…",
        Some(Operation::SignUp) => "Criando conta…",
        Some(Operation::SendRecovery) => "Enviando…",
        Some(_) => "Aguarde…",
        None => label,
    };
    let submit = primary_button(ui, button_text, enabled, t, m) || (enabled && enter);
    if !tabbed {
        back_link(auth, ui, t, m, &mut navigate);
    }
    if navigate.is_some() {
        return finish_navigation(auth, ui, navigate);
    }
    submit.then_some(command)
}

fn copy(auth: &Auth) -> (&'static str, &'static str, &'static str, &'static str) {
    match auth.screen {
        Screen::SignIn => ("Acesse seu espaço", "Continue seu próximo projeto.", "Entrar no DesignFlow", "auth.signIn"),
        Screen::SignUp => ("Acesse seu espaço", "Continue seu próximo projeto.", "Criar minha conta", "auth.signUp"),
        Screen::VerifySignup => ("Confira seu e-mail", "Seu espaço está quase pronto.", "Confirmar código", "auth.verifySignup"),
        Screen::Recovery => ("Recupere seu acesso", "Vamos ajudar você a voltar a criar.", "Enviar recuperação", "auth.sendRecovery"),
        Screen::VerifyRecovery if !auth.email_codes_enabled => {
            ("Recupere seu acesso", "Copie o link do e-mail sem abri-lo e cole abaixo.", "Continuar", "auth.verifyRecovery")
        }
        Screen::VerifyRecovery => ("Recupere seu acesso", "Informe o código recebido por e-mail.", "Verificar código", "auth.verifyRecovery"),
        Screen::NewPassword => ("Uma nova senha", "Escolha uma senha para voltar a criar.", "Salvar nova senha", "auth.updatePassword"),
    }
}

fn account_tabs(auth: &Auth, ui: &mut Ui, t: &Tokens, m: Metrics) -> Option<Screen> {
    let mut target = None;
    ui.add_enabled_ui(auth.pending.is_none(), |ui| {
        let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0 * m.scale), Sense::hover());
        ui.painter().line_segment([bar.left_bottom(), bar.right_bottom()], Stroke::new(1.0, t.border));
        let mut left = bar.left();
        for (screen, label, width) in [(Screen::SignIn, "Entrar", 112.0), (Screen::SignUp, "Criar conta", 144.0)] {
            let selected = auth.screen == screen;
            let rect = Rect::from_min_size(pos2(left, bar.top()), vec2((width * m.scale).min(bar.width() * 0.5), bar.height()));
            let response = ui.interact(rect, ui.scope_id().with(("designflow_tab", screen as u8)), Sense::click());
            crate::a11y::tab(&response, label, selected);
            let color = if selected || response.hovered() { t.accent } else { t.text_secondary };
            ui.painter().text(
                rect.center() - vec2(0.0, 3.0),
                Align2::CENTER_CENTER,
                label,
                if selected { semibold(m.body) } else { regular(m.body) },
                color,
            );
            if selected {
                ui.painter().line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(2.0, t.accent));
            }
            if response.has_focus() {
                ui.painter().rect_stroke(rect.shrink(3.0), 4, Stroke::new(1.0, t.focus), StrokeKind::Inside);
            }
            if response.clicked() && !selected {
                target = Some(screen);
            }
            left = rect.right();
        }
    });
    target
}

fn fields(auth: &mut Auth, ui: &mut Ui, t: &Tokens, m: Metrics) -> bool {
    let mut enter = false;
    if auth.screen != Screen::NewPassword {
        let editable = matches!(auth.screen, Screen::SignIn | Screen::SignUp | Screen::Recovery);
        ui.add_enabled_ui(editable, |ui| enter |= field(ui, "E-mail", "seu@email.com", &mut auth.email, None, 254, t, m));
    }
    if matches!(auth.screen, Screen::VerifySignup | Screen::VerifyRecovery) {
        m.gap(ui, 24.0);
        if auth.screen == Screen::VerifyRecovery && !auth.email_codes_enabled {
            enter |= field(ui, "Link de recuperação", "Cole o link recebido por e-mail", &mut auth.code, Some(&mut auth.reveal), 4096, t, m);
        } else {
            enter |= field(ui, "Código de verificação", "Código recebido por e-mail", &mut auth.code, None, 10, t, m);
        }
    }
    if matches!(auth.screen, Screen::SignIn | Screen::SignUp | Screen::NewPassword) {
        if auth.screen != Screen::NewPassword {
            m.gap(ui, 24.0);
        }
        let label = if auth.screen == Screen::NewPassword { "Nova senha" } else { "Senha" };
        enter |= field(ui, label, "Digite sua senha", &mut auth.password, Some(&mut auth.reveal), 1024, t, m);
    }
    if matches!(auth.screen, Screen::SignUp | Screen::NewPassword) {
        m.gap(ui, 24.0);
        enter |= field(ui, "Confirmar senha", "Repita sua senha", &mut auth.confirmation, Some(&mut auth.reveal), 1024, t, m);
        m.gap(ui, 12.0);
        ui.label(RichText::new("Use pelo menos 8 caracteres.").font(regular(m.body - 2.0)).color(t.text_secondary));
    }
    enter
}

fn field(ui: &mut Ui, label: &str, hint: &str, value: &mut String, reveal: Option<&mut bool>, limit: usize, t: &Tokens, m: Metrics) -> bool {
    ui.label(RichText::new(label).font(semibold(m.body)).color(t.text));
    m.gap(ui, 8.0);
    let secret = reveal.is_some();
    let password = reveal.as_deref().is_some_and(|visible| !*visible);
    let field_frame = egui::Frame::new()
        .fill(t.field)
        .stroke(Stroke::new(1.0, t.border_strong))
        .corner_radius(8)
        .inner_margin(Margin::symmetric(16, ((m.field - 24.0) * 0.5) as i8));
    let mut edit = None;
    let outer = field_frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.set_height(24.0);
            let width = (ui.available_width() - if secret { 40.0 } else { 0.0 }).max(24.0);
            let response = ui.add_sized(
                vec2(width, 24.0),
                TextEdit::singleline(value)
                    .id_salt(label)
                    .hint_text(RichText::new(hint).font(regular(m.body)).color(t.text_secondary))
                    .password(password)
                    .char_limit(limit)
                    .font(regular(m.body))
                    .margin(Margin::ZERO)
                    .frame(egui::Frame::NONE),
            );
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, ui.is_enabled(), label));
            if let Some(visible) = reveal {
                let (rect, eye) = ui.allocate_exact_size(vec2(32.0, 24.0), Sense::click());
                let name = match (label == "Link de recuperação", *visible) {
                    (true, true) => "Ocultar link",
                    (true, false) => "Mostrar link",
                    (false, true) => "Ocultar senha",
                    (false, false) => "Mostrar senha",
                };
                crate::a11y::button(&eye, name);
                icons::paint(
                    ui.painter(),
                    Rect::from_center_size(rect.center(), vec2(32.0, 32.0)),
                    if *visible { Icon::EyeOff } else { Icon::Eye },
                    t.text_secondary,
                );
                if eye.has_focus() {
                    ui.painter().rect_stroke(rect.expand(2.0), 4, Stroke::new(1.0, t.focus), StrokeKind::Inside);
                }
                if eye.clicked() {
                    *visible = !*visible;
                    response.request_focus();
                }
                eye.on_hover_text(name);
            }
            edit = Some(response);
        });
    });
    if let Some(response) = edit {
        if response.has_focus() {
            ui.painter().rect_stroke(outer.response.rect, 8, Stroke::new(2.0, t.focus), StrokeKind::Inside);
        }
        return response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    }
    false
}

fn primary_button(ui: &mut Ui, text: &str, enabled: bool, t: &Tokens, m: Metrics) -> bool {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), (60.0 * m.scale).max(48.0)), Sense::click());
        crate::a11y::button(&response, text);
        let tint = if response.is_pointer_button_down_on() {
            0.9
        } else if response.hovered() {
            0.96
        } else {
            1.0
        };
        gradient(ui, rect, Color32::from_rgb(0x9C, 0x3B, 0xFF).gamma_multiply(tint), t.accent.gamma_multiply(tint));
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, semibold(m.body), t.accent_on);
        if response.has_focus() {
            ui.painter().rect_stroke(rect.expand(3.0), 10, Stroke::new(2.0, t.focus), StrokeKind::Outside);
        }
        response.clicked()
    })
    .inner
}

fn gradient(ui: &Ui, rect: Rect, top: Color32, bottom: Color32) {
    let radius = 8.0_f32.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let mut mesh = egui::Mesh::default();
    let color = |y: f32| top.lerp_to_gamma(bottom, ((y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0));
    mesh.colored_vertex(rect.center(), color(rect.center().y));
    for (center, start) in [
        (pos2(rect.right() - radius, rect.top() + radius), -std::f32::consts::FRAC_PI_2),
        (pos2(rect.right() - radius, rect.bottom() - radius), 0.0),
        (pos2(rect.left() + radius, rect.bottom() - radius), std::f32::consts::FRAC_PI_2),
        (pos2(rect.left() + radius, rect.top() + radius), std::f32::consts::PI),
    ] {
        for step in 0..=8 {
            let angle = start + step as f32 * std::f32::consts::FRAC_PI_2 / 8.0;
            let point = center + vec2(angle.cos(), angle.sin()) * radius;
            mesh.colored_vertex(point, color(point.y));
        }
    }
    let count = mesh.vertices.len().saturating_sub(1) as u32;
    for vertex in 1..=count {
        mesh.add_triangle(0, vertex, if vertex == count { 1 } else { vertex + 1 });
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}

fn text_link(ui: &mut Ui, text: &str, enabled: bool, t: &Tokens, m: Metrics) -> bool {
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).font(regular(m.body - 1.0)).color(t.accent_text)).frame(false).min_size(vec2(0.0, 24.0)),
    )
    .clicked()
}
fn back_link(auth: &Auth, ui: &mut Ui, t: &Tokens, m: Metrics, target: &mut Option<Screen>) {
    m.gap(ui, 22.0);
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        if text_link(ui, "Voltar para entrar", auth.pending.is_none(), t, m) {
            *target = Some(Screen::SignIn);
        }
    });
}
fn finish_navigation(auth: &mut Auth, ui: &Ui, target: Option<Screen>) -> Option<&'static str> {
    if let Some(screen) = target {
        auth.navigate(screen);
        ui.ctx().request_repaint();
    }
    None
}
fn notice(ui: &mut Ui, t: &Tokens, m: Metrics, error: bool, message: &str) {
    ui.label(RichText::new(message).font(regular(m.body - 2.0)).color(if error { t.danger } else { t.text_secondary }));
}
