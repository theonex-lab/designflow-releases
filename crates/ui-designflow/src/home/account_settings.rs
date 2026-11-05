//! Account and preferences forms for the native hub. Persistence belongs to `workspace::Store`.
use crate::icons::{self, Icon};
use crate::{
    FigApp, a11y,
    theme::{self, Tokens},
    workspace::{Preferences, Store},
};
use egui::{Align, Align2, Color32, Layout, Margin, RichText, Stroke, StrokeKind, Ui, vec2};

#[derive(Default)]
pub(super) struct State {
    initialized: bool,
    account_key: String,
    profile_name: String,
    editing_profile: bool,
    focus_name: bool,
    preferences: Option<Preferences>,
    section: usize,
    pending: Option<Pending>,
    feedback: Feedback,
}

enum Pending {
    Profile(String),
    Preferences(Preferences),
}

#[derive(Default)]
enum Feedback {
    #[default]
    Idle,
    Saved(String),
    Error(String),
}

impl State {
    fn prepare(&mut self, app: &FigApp, store: &Store) {
        let key = app.auth.as_ref().and_then(crate::auth::Auth::account_email).unwrap_or_default();
        if !self.initialized || self.account_key != key {
            *self = Self {
                initialized: true,
                account_key: key.to_owned(),
                profile_name: store.data.profile.name.clone(),
                preferences: Some(store.data.preferences.clone()),
                ..Self::default()
            };
        }
    }

    fn commit_pending(&mut self, app: &mut FigApp, store: &mut Store) {
        let Some(pending) = self.pending.take() else { return };
        match pending {
            Pending::Profile(name) => match store.save_profile(&name) {
                Ok(()) => {
                    self.profile_name = store.data.profile.name.clone();
                    self.editing_profile = false;
                    app.session.prefs.user_name = self.profile_name.clone();
                    app.session.prefs_unsaved = true;
                    self.feedback = Feedback::Saved("Perfil atualizado neste computador.".into());
                }
                Err(error) => self.feedback = Feedback::Error(error),
            },
            Pending::Preferences(preferences) => match store.save_preferences(preferences) {
                Ok(()) => {
                    let preferences = &store.data.preferences;
                    app.session.prefs.auto_save = preferences.autosave;
                    app.session.prefs.theme = preferences.theme.clone();
                    app.session.prefs.language = preferences.locale.clone();
                    app.session.prefs.ui_scale = f64::from(preferences.ui_scale_percent) / 100.0;
                    app.session.prefs.start_on_home = true;
                    app.session.prefs_unsaved = true;
                    self.preferences = Some(preferences.clone());
                    self.feedback = Feedback::Saved("Alterações salvas neste computador.".into());
                }
                Err(error) => self.feedback = Feedback::Error(error),
            },
        }
    }
}

/// Body only: the hub renders the shared page heading and navigation.
pub(super) fn show_account(app: &mut FigApp, ui: &mut Ui, store: &mut Store, state: &mut State, tokens: &Tokens) {
    state.prepare(app, store);
    state.commit_pending(app, store);
    let email = app.auth.as_ref().and_then(crate::auth::Auth::account_email).map(str::to_owned);
    let busy = state.pending.is_some();
    ui.set_max_width(ui.available_width().min(1120.0));
    card(ui, tokens, |ui| {
        ui.set_width(ui.available_width());
        if state.editing_profile {
            title(ui, "Editar perfil", tokens);
            ui.add_space(24.0);
            let response = field(ui, "Nome", "Como você quer aparecer", &mut state.profile_name, tokens);
            if state.focus_name {
                response.request_focus();
                state.focus_name = false;
            }
            let valid = valid_profile_name(&state.profile_name);
            if !valid {
                small_error(ui, "Informe um nome de até 120 caracteres.", tokens);
            }
            ui.add_space(16.0);
            readonly_email(ui, email.as_deref(), tokens);
            ui.add_space(24.0);
            ui.horizontal_wrapped(|ui| {
                if action(ui, "Salvar perfil", true, valid && !busy && email.is_some(), tokens) {
                    state.pending = Some(Pending::Profile(state.profile_name.trim().to_owned()));
                    state.feedback = Feedback::Idle;
                    ui.ctx().request_repaint();
                }
                if action(ui, "Cancelar", false, !busy, tokens) {
                    state.profile_name = store.data.profile.name.clone();
                    state.editing_profile = false;
                    state.feedback = Feedback::Idle;
                }
            });
        } else {
            let name = if store.data.profile.name.is_empty() { "Nome não informado" } else { &store.data.profile.name };
            let initial = store
                .data
                .profile
                .name
                .chars()
                .find(|c| !c.is_whitespace())
                .or_else(|| email.as_deref().and_then(|value| value.chars().next()))
                .map(|value| value.to_uppercase().collect::<String>())
                .unwrap_or_else(|| "?".into());
            let compact = ui.available_width() < 480.0;
            let mut content = |ui: &mut Ui| {
                ui.add(egui::Label::new(RichText::new(name).font(theme::designflow_semibold(24.0)).color(tokens.text)).wrap());
                ui.add_space(6.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(email.as_deref().unwrap_or("Nenhuma conta autenticada"))
                            .font(theme::designflow_regular(16.0))
                            .color(tokens.text_secondary),
                    )
                    .wrap(),
                );
                ui.add_space(20.0);
                if action(ui, "Editar perfil", false, email.is_some() && !busy, tokens) {
                    state.editing_profile = true;
                    state.focus_name = true;
                    state.feedback = Feedback::Idle;
                    ui.ctx().request_repaint();
                }
            };
            if compact {
                avatar(ui, &initial, 80.0, tokens);
                ui.add_space(20.0);
                content(ui);
            } else {
                ui.horizontal(|ui| {
                    avatar(ui, &initial, 112.0, tokens);
                    ui.add_space(24.0);
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width());
                        content(ui);
                    });
                });
            }
        }
    });
    ui.add_space(24.0);
    card(ui, tokens, |ui| {
        title(ui, "Acesso", tokens);
        ui.add_space(8.0);
        description(ui, "Sua sessão usa a conta salva neste computador.", tokens);
        ui.add_space(20.0);
        if action(ui, "Sair da conta", false, email.is_some() && !busy, tokens) {
            app.run("auth.signOut", serde_json::json!({}));
            if app.auth.as_ref().is_some_and(crate::auth::Auth::unlocked) {
                state.feedback = Feedback::Error(app.ui.last_error.clone().unwrap_or_else(|| "Não foi possível sair da conta.".into()));
            }
        }
    });
    feedback(ui, state, tokens);
}

/// Body only: controls mutate a draft and apply it only after the local adapter confirms saving.
pub(super) fn show_settings(app: &mut FigApp, ui: &mut Ui, store: &mut Store, state: &mut State, tokens: &Tokens) {
    state.prepare(app, store);
    state.commit_pending(app, store);
    ui.set_max_width(ui.available_width().min(1120.0));
    let busy = state.pending.is_some();
    let mut draft = state.preferences.clone().unwrap_or_else(|| store.data.preferences.clone());
    let before = draft.clone();
    if ui.available_width() >= 900.0 {
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2(232.0, 0.0), Layout::top_down(Align::Min), |ui| {
                egui::Frame::new().fill(tokens.panel).stroke(Stroke::new(1.0, tokens.border)).corner_radius(16).inner_margin(16).show(ui, |ui| {
                    ui.set_min_height(440.0);
                    settings_navigation(ui, &mut state.section, true, tokens);
                });
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                settings_body(ui, state.section, &mut draft, busy, tokens);
            });
        });
    } else {
        ui.horizontal_wrapped(|ui| settings_navigation(ui, &mut state.section, false, tokens));
        ui.add_space(24.0);
        settings_body(ui, state.section, &mut draft, busy, tokens);
    }
    if before != draft {
        state.feedback = Feedback::Idle;
    }
    state.preferences = Some(draft.clone());
    let dirty = draft != store.data.preferences;
    let valid = valid_preferences(&draft);
    ui.add_space(20.0);
    if !valid {
        small_error(ui, "Use um nome de espaço com 1 a 80 caracteres e selecione preferências válidas (tamanho de 75% a 300%).", tokens);
    }
    ui.with_layout(Layout::right_to_left(Align::Center).with_main_wrap(true), |ui| {
        if action(ui, "Salvar alterações", true, dirty && valid && !busy, tokens) {
            state.pending = Some(Pending::Preferences(draft));
            state.feedback = Feedback::Idle;
            ui.ctx().request_repaint();
        }
        if dirty && !busy && action(ui, "Descartar alterações", false, true, tokens) {
            state.preferences = Some(store.data.preferences.clone());
            state.feedback = Feedback::Idle;
        }
        if dirty && !busy {
            description(ui, "Há alterações não salvas.", tokens);
        }
    });
    feedback(ui, state, tokens);
}

fn settings_navigation(ui: &mut Ui, section: &mut usize, vertical: bool, tokens: &Tokens) {
    for (index, name) in ["Geral", "Aparência", "Notificações", "Atalhos", "Idioma", "Privacidade"].into_iter().enumerate() {
        let selected = *section == index;
        let font = theme::designflow_regular(15.0);
        let text_width = ui.painter().layout_no_wrap(name.to_owned(), font.clone(), tokens.text).size().x;
        let (rect, response) =
            ui.allocate_exact_size(vec2(if vertical { ui.available_width() } else { text_width + 64.0 }, 48.0), egui::Sense::click());
        let color = if selected { tokens.accent_text } else { tokens.text };
        ui.painter().rect_filled(
            rect,
            12,
            if selected {
                tokens.selected_row
            } else if response.hovered() {
                tokens.hover
            } else {
                tokens.panel
            },
        );
        if response.has_focus() {
            ui.painter().rect_stroke(rect, 12, Stroke::new(2.0, tokens.focus), StrokeKind::Inside);
        }
        settings_icon(ui, egui::Rect::from_center_size(egui::pos2(rect.left() + 24.0, rect.center().y), vec2(20.0, 20.0)), index, color);
        ui.painter().text(egui::pos2(rect.left() + 46.0, rect.center().y), Align2::LEFT_CENTER, name, font, color);
        a11y::toggle(&response, name, selected);
        if response.clicked() {
            *section = index;
        }
        if vertical {
            ui.add_space(8.0);
        }
    }
}

fn settings_icon(ui: &Ui, rect: egui::Rect, index: usize, color: Color32) {
    let painter = ui.painter();
    let center = rect.center();
    let stroke = Stroke::new(1.6, color);
    match index {
        0 => {
            let outline = (0..32)
                .map(|step| {
                    let angle = step as f32 * std::f32::consts::TAU / 32.0;
                    let radius = if matches!(step % 4, 0 | 3) { 8.6 } else { 6.8 };
                    center + vec2(angle.cos(), angle.sin()) * radius
                })
                .collect();
            painter.add(egui::Shape::closed_line(outline, stroke));
            painter.circle_stroke(center, 2.8, stroke);
        }
        1 => {
            painter.rect_stroke(egui::Rect::from_center_size(center - vec2(0.0, 2.0), vec2(17.0, 12.0)), 2, stroke, StrokeKind::Inside);
            painter.line_segment([center + vec2(0.0, 4.0), center + vec2(0.0, 8.0)], stroke);
            painter.line_segment([center + vec2(-5.0, 8.0), center + vec2(5.0, 8.0)], stroke);
        }
        2 => icons::paint(painter, rect, Icon::Bell, color),
        3 => {
            painter.rect_stroke(egui::Rect::from_center_size(center, vec2(18.0, 14.0)), 2, stroke, StrokeKind::Inside);
            for y in [-3.0, 0.0] {
                for x in [-5.5, -1.8, 1.8, 5.5] {
                    painter.circle_filled(center + vec2(x, y), 0.7, color);
                }
            }
            painter.line_segment([center + vec2(-4.0, 3.5), center + vec2(4.0, 3.5)], stroke);
        }
        4 => {
            painter.circle_stroke(center, 8.5, stroke);
            painter.line_segment([center + vec2(-8.5, 0.0), center + vec2(8.5, 0.0)], stroke);
            let meridian = (0..24)
                .map(|step| {
                    let angle = step as f32 * std::f32::consts::TAU / 24.0;
                    center + vec2(angle.cos() * 3.8, angle.sin() * 8.5)
                })
                .collect();
            painter.add(egui::Shape::closed_line(meridian, stroke));
        }
        _ => {
            painter.add(egui::Shape::closed_line(
                [vec2(0.0, -9.0), vec2(7.0, -6.0), vec2(6.0, 3.0), vec2(0.0, 9.0), vec2(-6.0, 3.0), vec2(-7.0, -6.0)]
                    .into_iter()
                    .map(|point| center + point)
                    .collect(),
                stroke,
            ));
            painter.line_segment([center + vec2(0.0, -4.5), center + vec2(0.0, 4.5)], stroke);
        }
    }
}

fn settings_body(ui: &mut Ui, section: usize, draft: &mut Preferences, busy: bool, tokens: &Tokens) {
    card(ui, tokens, |ui| {
        ui.add_enabled_ui(!busy, |ui| match section {
            0 => general(ui, draft, tokens),
            1 => appearance(ui, draft, tokens),
            2 => {
                title(ui, "Notificações", tokens);
                ui.add_space(12.0);
                description(ui, "O envio de notificações por e-mail não está disponível para contas locais.", tokens);
            }
            3 => shortcuts(ui, tokens),
            4 => language(ui, draft, tokens),
            _ => {
                title(ui, "Privacidade", tokens);
                ui.add_space(12.0);
                description(ui, "Seu espaço pessoal e seu perfil são armazenados neste computador. Não há sincronização de conta ou armazenamento em nuvem configurados.", tokens);
                ui.add_space(16.0);
                description(ui, "Ações de publicação e compartilhamento do editor têm seus próprios avisos antes de enviar arquivos.", tokens);
            }
        });
    });
}

fn general(ui: &mut Ui, draft: &mut Preferences, t: &Tokens) {
    setting_row(ui, "Nome do espaço", "Um nome para organizar seus projetos locais.", t, |ui| {
        field_control(ui, "Nome do espaço", "Meu espaço", &mut draft.workspace_name);
    });
    separator(ui, t);
    setting_row(ui, "Salvar automaticamente", "Salva arquivos que já têm um caminho neste computador.", t, |ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            autosave_switch(ui, &mut draft.autosave, t);
        });
    });
    separator(ui, t);
    setting_row(ui, "Tela inicial", "Escolha a página aberta ao entrar no seu espaço.", t, |ui| {
        craft_ui::ComboBox::from_id_salt("designflow_default_route")
            .selected_text(route_name(&draft.default_route))
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for route in ["/", "/projetos", "/modelos", "/importar", "/recursos", "/biblioteca", "/lixeira", "/conta", "/configuracoes"] {
                    ui.selectable_value(&mut draft.default_route, route.to_owned(), route_name(route));
                }
            });
    });
}

fn appearance(ui: &mut Ui, draft: &mut Preferences, t: &Tokens) {
    title(ui, "Tema", t);
    ui.add_space(8.0);
    description(ui, "Escolha a aparência do espaço pessoal e do editor.", t);
    ui.add_space(24.0);
    ui.horizontal_wrapped(|ui| {
        for (value, label) in [("light", "Claro"), ("dark", "Escuro"), ("system", "Sistema")] {
            let response = ui.add_sized(
                vec2(116.0, 44.0),
                egui::Button::new(RichText::new(label).font(theme::designflow_regular(15.0)))
                    .wrap_mode(egui::TextWrapMode::Extend)
                    .fill(if draft.theme == value { t.selected_row } else { t.field })
                    .corner_radius(12),
            );
            a11y::radio(&response, label, draft.theme == value);
            if response.clicked() {
                draft.theme = value.to_owned();
            }
        }
    });
    separator(ui, t);
    setting_row(ui, "Tamanho da interface", "Ajusta menus e textos. O zoom do canvas continua independente.", t, |ui| {
        craft_ui::ComboBox::from_id_salt("designflow_ui_scale")
            .selected_text(format!("{}%", draft.ui_scale_percent))
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for percent in [75, 100, 125, 150, 175, 200, 250, 300] {
                    ui.selectable_value(&mut draft.ui_scale_percent, percent, format!("{percent}%"));
                }
            });
    });
}

fn language(ui: &mut Ui, draft: &mut Preferences, t: &Tokens) {
    title(ui, "Idioma do editor", t);
    ui.add_space(8.0);
    description(ui, "As telas do espaço pessoal permanecem em português.", t);
    ui.add_space(24.0);
    let name = if draft.locale == "system" {
        "Sistema"
    } else {
        crate::i18n::languages().iter().find(|language| language.code == draft.locale).map_or(draft.locale.as_str(), |language| language.name)
    }
    .to_owned();
    craft_ui::ComboBox::from_id_salt("designflow_editor_language").selected_text(name).width(ui.available_width().min(400.0)).show_ui(ui, |ui| {
        ui.selectable_value(&mut draft.locale, "system".to_owned(), "Sistema");
        for language in crate::i18n::languages() {
            ui.selectable_value(&mut draft.locale, language.code.to_owned(), language.name);
        }
    });
}

fn shortcuts(ui: &mut Ui, t: &Tokens) {
    title(ui, "Atalhos essenciais", t);
    ui.add_space(20.0);
    for (action, shortcut) in [
        ("Selecionar", "V"),
        ("Mão / navegar", "H"),
        ("Quadro", "F"),
        ("Texto", "T"),
        ("Retângulo", "R"),
        ("Elipse", "O"),
        ("Salvar", "Ctrl S"),
        ("Desfazer", "Ctrl Z"),
        ("Refazer", "Ctrl Shift Z"),
        ("Buscar", "Ctrl K"),
    ] {
        ui.horizontal(|ui| {
            ui.label(RichText::new(action).font(theme::designflow_regular(15.0)).color(t.text));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(shortcut).font(egui::FontId::monospace(13.0)).color(t.text_secondary));
            });
        });
        ui.add_space(12.0);
    }
}

fn route_name(route: &str) -> &'static str {
    match route {
        "/projetos" => "Projetos",
        "/modelos" => "Modelos",
        "/importar" => "Importar",
        "/recursos" => "Recursos",
        "/biblioteca" => "Biblioteca",
        "/lixeira" => "Lixeira",
        "/conta" => "Minha conta",
        "/configuracoes" => "Configurações",
        _ => "Início",
    }
}

fn valid_profile_name(name: &str) -> bool {
    !name.trim().is_empty() && name.trim().chars().count() <= 120 && !name.chars().any(char::is_control)
}

fn valid_preferences(prefs: &Preferences) -> bool {
    !prefs.workspace_name.trim().is_empty()
        && prefs.workspace_name.trim().chars().count() <= 80
        && !prefs.workspace_name.chars().any(char::is_control)
        && matches!(prefs.theme.as_str(), "light" | "dark" | "system")
        && (75..=300).contains(&prefs.ui_scale_percent)
        && matches!(
            prefs.default_route.as_str(),
            "/" | "/projetos" | "/modelos" | "/importar" | "/recursos" | "/biblioteca" | "/lixeira" | "/conta" | "/configuracoes"
        )
        && (prefs.locale == "system" || crate::i18n::languages().iter().any(|language| language.code == prefs.locale))
}

fn card(ui: &mut Ui, t: &Tokens, content: impl FnOnce(&mut Ui)) {
    egui::Frame::new().fill(t.panel).stroke(Stroke::new(1.0, t.border)).corner_radius(16).inner_margin(Margin::same(24)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        content(ui);
    });
}

fn title(ui: &mut Ui, text: &str, t: &Tokens) {
    ui.label(RichText::new(text).font(theme::designflow_semibold(20.0)).color(t.text));
}

fn label(ui: &mut Ui, text: &str, t: &Tokens) {
    ui.add(egui::Label::new(RichText::new(text).font(theme::designflow_semibold(16.0)).color(t.text)).wrap());
}

fn description(ui: &mut Ui, text: &str, t: &Tokens) {
    ui.add(egui::Label::new(RichText::new(text).font(theme::designflow_regular(14.0)).color(t.text_secondary)).wrap());
}

fn readonly_email(ui: &mut Ui, email: Option<&str>, t: &Tokens) {
    label(ui, "E-mail da conta", t);
    ui.label(RichText::new(email.unwrap_or("Nenhuma conta autenticada")).font(theme::designflow_regular(16.0)).color(t.text));
}

fn field(ui: &mut Ui, name: &str, hint: &str, value: &mut String, t: &Tokens) -> egui::Response {
    label(ui, name, t);
    ui.add_space(6.0);
    field_control(ui, name, hint, value)
}

fn field_control(ui: &mut Ui, name: &str, hint: &str, value: &mut String) -> egui::Response {
    let response = ui.add_sized(
        vec2(ui.available_width(), 44.0),
        egui::TextEdit::singleline(value)
            .id_salt(("workspace_field", name))
            .font(theme::designflow_regular(15.0))
            .hint_text(hint)
            .char_limit(if name == "Nome do espaço" { 80 } else { 120 })
            .margin(Margin::symmetric(12, 12)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, ui.is_enabled(), name));
    response
}

fn action(ui: &mut Ui, label: &str, primary: bool, enabled: bool, t: &Tokens) -> bool {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).font(theme::designflow_semibold(15.0)).color(if primary { Color32::WHITE } else { t.text }))
            .wrap_mode(egui::TextWrapMode::Extend)
            .fill(if primary { t.accent } else { t.panel })
            .stroke(Stroke::new(1.0, if primary { t.accent } else { t.border }))
            .min_size(vec2(0.0, 44.0))
            .corner_radius(12),
    );
    a11y::button(&response, label);
    response.clicked()
}

fn avatar(ui: &mut Ui, initial: &str, size: f32, t: &Tokens) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), size * 0.5, t.selected_row);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, initial, theme::designflow_regular(size * 0.42), t.accent_text);
}

fn autosave_switch(ui: &mut Ui, value: &mut bool, t: &Tokens) {
    let (rect, mut response) = ui.allocate_exact_size(vec2(52.0, 44.0), egui::Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let track = egui::Rect::from_center_size(rect.center(), vec2(52.0, 30.0));
    ui.painter().rect_filled(track, 15, if *value { t.accent } else { t.border });
    let knob_x = if *value { track.right() - 15.0 } else { track.left() + 15.0 };
    ui.painter().circle_filled(egui::pos2(knob_x, track.center().y), 11.0, Color32::WHITE);
    if response.has_focus() {
        ui.painter().rect_stroke(track.expand(3.0), 18, Stroke::new(2.0, t.focus), StrokeKind::Outside);
    }
    a11y::checkbox(&response, "Ativar salvamento automático", *value);
    response.on_hover_text(if *value { "Salvamento automático ativado" } else { "Salvamento automático desativado" });
}

fn setting_row(ui: &mut Ui, title: &str, description_text: &str, t: &Tokens, control: impl FnOnce(&mut Ui)) {
    if ui.available_width() >= 660.0 {
        ui.columns(2, |columns| {
            label(&mut columns[0], title, t);
            description(&mut columns[0], description_text, t);
            control(&mut columns[1]);
        });
    } else {
        label(ui, title, t);
        description(ui, description_text, t);
        ui.add_space(14.0);
        control(ui);
    }
}

fn separator(ui: &mut Ui, _t: &Tokens) {
    ui.add_space(20.0);
    ui.separator();
    ui.add_space(20.0);
}

fn small_error(ui: &mut Ui, message: &str, t: &Tokens) {
    let response = ui.label(RichText::new(message).font(theme::designflow_regular(13.0)).color(t.danger));
    a11y::status(&response, "Verifique os dados", message);
}

fn feedback(ui: &mut Ui, state: &State, t: &Tokens) {
    ui.add_space(18.0);
    if state.pending.is_some() {
        let response = ui.label(RichText::new("Salvando alterações…").font(theme::designflow_regular(14.0)).color(t.text_secondary));
        a11y::status(&response, "Salvando", "Aguarde a confirmação do armazenamento local.");
    } else {
        match &state.feedback {
            Feedback::Idle => {}
            Feedback::Saved(message) => {
                let response = ui.label(RichText::new(message).font(theme::designflow_regular(14.0)).color(t.success));
                a11y::status(&response, "Alterações salvas", message);
            }
            Feedback::Error(message) => small_error(ui, message, t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_save_retains_drafts_and_applies_native_preferences_only_after_success() {
        let directory = std::env::temp_dir().join(format!(
            "designflow-account-settings-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let mut store = Store::from_directory(directory.clone()).unwrap();
        let mut app = FigApp::new(designflow_engine::Session::empty(), Box::new(crate::NoServices));
        let native_before = app.session.prefs.clone();
        let draft = Preferences { autosave: true, theme: "dark".into(), locale: "pt-BR".into(), ui_scale_percent: 200, ..Preferences::default() };
        let mut state = State { preferences: Some(draft.clone()), pending: Some(Pending::Preferences(draft.clone())), ..State::default() };
        let mut second_window = Store::from_directory(directory.clone()).unwrap();
        second_window.save_profile("Perfil mais recente").unwrap();

        state.commit_pending(&mut app, &mut store);
        assert!(matches!(state.feedback, Feedback::Error(_)));
        assert_eq!(state.preferences.as_ref(), Some(&draft));
        assert_eq!(app.session.prefs, native_before);
        assert_eq!(Store::from_directory(directory.clone()).unwrap().data.profile.name, "Perfil mais recente");

        store.reload().unwrap();
        state.pending = Some(Pending::Preferences(draft.clone()));
        state.commit_pending(&mut app, &mut store);
        assert!(matches!(state.feedback, Feedback::Saved(_)));
        assert_eq!(store.data.preferences, draft);
        assert_eq!(app.session.prefs.ui_scale, 2.0);
        assert_eq!(app.session.prefs.theme, "dark");
        assert_eq!(app.session.prefs.language, "pt-BR");
        assert!(app.session.prefs.auto_save);

        second_window.reload().unwrap();
        second_window.save_profile("Nome em outra janela").unwrap();
        state.profile_name = "Nome pendente".into();
        state.editing_profile = true;
        state.pending = Some(Pending::Profile(state.profile_name.clone()));
        state.commit_pending(&mut app, &mut store);
        assert!(matches!(state.feedback, Feedback::Error(_)));
        assert!(state.editing_profile);
        assert_eq!(state.profile_name, "Nome pendente");
        assert_eq!(app.session.prefs.user_name, native_before.user_name);
        store.reload().unwrap();
        state.pending = Some(Pending::Profile(state.profile_name.clone()));
        state.commit_pending(&mut app, &mut store);
        assert!(!state.editing_profile);
        assert_eq!(app.session.prefs.user_name, "Nome pendente");
        assert_eq!(Store::from_directory(directory.clone()).unwrap().data.profile.name, "Nome pendente");

        assert_eq!(directory.parent(), Some(std::env::temp_dir().as_path()));
        assert!(directory.file_name().unwrap().to_string_lossy().starts_with("designflow-account-settings-test-"));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
