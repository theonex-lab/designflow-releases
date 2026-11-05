//! Native routes and controlled page state for the DesignFlow v2 hub.
use super::{account_settings, collections, hub_widgets as w};
use crate::{
    FigApp, a11y,
    icons::{self, Icon},
    theme::{self, Tokens, system as metrics},
    workspace::{self, Store},
};
use egui::{Align, Align2, Color32, Id, Layout, Margin, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Route {
    #[default]
    Home,
    Projects,
    Templates,
    Import,
    Resources,
    Library,
    Trash,
    Account,
    Settings,
}
impl Route {
    pub fn path(self) -> &'static str {
        match self {
            Self::Home => "/",
            Self::Projects => "/projetos",
            Self::Templates => "/modelos",
            Self::Import => "/importar",
            Self::Resources => "/recursos",
            Self::Library => "/biblioteca",
            Self::Trash => "/lixeira",
            Self::Account => "/conta",
            Self::Settings => "/configuracoes",
        }
    }
    pub fn parse(path: &str) -> Option<Self> {
        Some(match path {
            "/" => Self::Home,
            "/projetos" => Self::Projects,
            "/modelos" => Self::Templates,
            "/importar" => Self::Import,
            "/recursos" => Self::Resources,
            "/biblioteca" => Self::Library,
            "/lixeira" => Self::Trash,
            "/conta" => Self::Account,
            "/configuracoes" => Self::Settings,
            _ => return None,
        })
    }
    fn label(self) -> &'static str {
        match self {
            Self::Home => "Início",
            Self::Projects => "Projetos",
            Self::Templates => "Modelos",
            Self::Import => "Importar",
            Self::Resources => "Recursos",
            Self::Library => "Biblioteca",
            Self::Trash => "Lixeira",
            Self::Account => "Minha conta",
            Self::Settings => "Configurações",
        }
    }
    fn icon(self) -> Icon {
        match self {
            Self::Home => Icon::Home,
            Self::Projects => Icon::Page,
            Self::Templates => Icon::Assets,
            Self::Import => Icon::Import,
            Self::Resources => Icon::Layers,
            Self::Library => Icon::Library,
            Self::Trash => Icon::Trash,
            Self::Account => Icon::User,
            Self::Settings => Icon::Adjust,
        }
    }
}

enum Modal {
    Create,
    Template(&'static str),
    Trash(String),
}
enum Completed {
    Loaded(bool),
    Project(String),
    Imported(String, Vec<String>),
}
struct Reply {
    store: Option<Store>,
    result: Result<Completed, String>,
}
struct Pending {
    receiver: Receiver<Reply>,
    label: &'static str,
}

pub struct State {
    pub route: Route,
    pub store: Option<Store>,
    account: String,
    loading_started: bool,
    pending: Option<Pending>,
    error: Option<String>,
    feedback: Option<String>,
    query: String,
    drawer: bool,
    drawer_opener: Option<Id>,
    list: bool,
    project_filter: usize,
    sort: usize,
    template_filter: usize,
    modal: Option<Modal>,
    modal_focus: bool,
    restore_focus: Option<Id>,
    name: String,
    editor_id: Option<String>,
    refresh_needed: bool,
    fit_pending: bool,
    settings: account_settings::State,
    collections: collections::State,
}
impl Default for State {
    fn default() -> Self {
        Self {
            route: Route::Home,
            store: None,
            account: String::new(),
            loading_started: false,
            pending: None,
            error: None,
            feedback: None,
            query: String::new(),
            drawer: false,
            drawer_opener: None,
            list: false,
            project_filter: 0,
            sort: 0,
            template_filter: 0,
            modal: None,
            modal_focus: false,
            restore_focus: None,
            name: "Sem título".into(),
            editor_id: None,
            refresh_needed: false,
            fit_pending: false,
            settings: Default::default(),
            collections: Default::default(),
        }
    }
}
impl State {
    fn go(&mut self, route: Route) {
        self.route = route;
        self.drawer = false;
        self.query.clear();
        self.error = None;
        self.feedback = None;
    }
    fn load(&mut self, ctx: &egui::Context) {
        let use_default_route = !self.loading_started;
        let account = self.account.clone();
        let (ctx, sender, receiver) = {
            let (tx, rx) = mpsc::channel();
            (ctx.clone(), tx, rx)
        };
        self.pending = Some(Pending { receiver, label: "Carregando seu espaço local…" });
        self.loading_started = true;
        self.error = None;
        std::thread::spawn(move || {
            let reply = match Store::load(&account) {
                Ok(store) => Reply { store: Some(store), result: Ok(Completed::Loaded(use_default_route)) },
                Err(error) => Reply { store: None, result: Err(error) },
            };
            let _ = sender.send(reply);
            ctx.request_repaint();
        });
    }
    fn job(&mut self, ctx: &egui::Context, label: &'static str, work: impl FnOnce(&mut Store) -> Result<Completed, String> + Send + 'static) {
        if self.pending.is_some() {
            return;
        }
        let Some(mut store) = self.store.clone() else { return };
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        self.pending = Some(Pending { receiver: rx, label });
        self.error = None;
        self.feedback = None;
        std::thread::spawn(move || {
            let result = work(&mut store);
            let _ = tx.send(Reply { store: Some(store), result });
            ctx.request_repaint();
        });
    }
    fn poll(&mut self, app: &mut FigApp) {
        let reply = self.pending.as_ref().and_then(|pending| match pending.receiver.try_recv() {
            Ok(r) => Some(r),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(Reply { store: None, result: Err("A operação local foi interrompida. Tente novamente.".into()) })
            }
        });
        let Some(reply) = reply else { return };
        self.pending = None;
        if let Some(store) = reply.store {
            self.store = Some(store);
        }
        match reply.result {
            Ok(Completed::Loaded(use_default_route)) => {
                if let Some(store) = &self.store {
                    let p = &store.data.preferences;
                    if use_default_route {
                        self.route = Route::parse(&p.default_route).unwrap_or_default();
                    }
                    app.session.prefs.auto_save = p.autosave;
                    app.session.prefs.theme = p.theme.clone();
                    app.session.prefs.language = p.locale.clone();
                    app.session.prefs.ui_scale = f64::from(p.ui_scale_percent) / 100.0;
                    app.session.prefs.user_name = store.data.profile.name.clone();
                }
            }
            Ok(Completed::Project(id)) => {
                self.close_modal(app.ui_context.as_ref());
                self.open_project(app, &id);
            }
            Ok(Completed::Imported(id, warnings)) => {
                let message = if warnings.is_empty() {
                    "Arquivo validado e importado neste computador.".into()
                } else {
                    format!("Importado com avisos: {}", warnings.join("; "))
                };
                self.feedback = Some(message.clone());
                app.toast(message);
                self.open_project(app, &id);
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn open_project(&mut self, app: &mut FigApp, id: &str) {
        let path = self.store.as_ref().ok_or("Espaço ainda não carregado.".to_owned()).and_then(|store| store.project_path(id));
        match path {
            Ok(path) => {
                if app.run_user("file.open", json!({"path":path.to_string_lossy()})).is_some() {
                    app.ui.home = false;
                    self.editor_id = Some(id.to_owned());
                    self.refresh_needed = true;
                    self.fit_pending = true;
                    let _ = app.run("view.rulers", json!({"on":true}));
                } else {
                    self.error = app.ui.last_error.clone();
                }
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn open_modal(&mut self, ui: &Ui, modal: Modal) {
        self.restore_focus = ui.ctx().memory(|m| m.focused());
        self.modal = Some(modal);
        self.modal_focus = true;
        self.error = None;
    }
    fn close_modal(&mut self, ctx: Option<&egui::Context>) {
        self.modal = None;
        self.modal_focus = false;
        if let (Some(ctx), Some(id)) = (ctx, self.restore_focus.take()) {
            ctx.memory_mut(|m| m.request_focus(id));
        }
    }
}

/// Native equivalent of route navigation; projects are opened only from this account's catalog.
pub fn navigate(app: &mut FigApp, path: &str) {
    let mut state = std::mem::take(&mut app.ui.home_view.designflow);
    if let Some(id) = path.strip_prefix("/editor/") {
        state.open_project(app, id);
    } else if let Some(route) = Route::parse(path) {
        state.go(route);
        if !app.ui.home && app.has_files() {
            state.refresh_needed = true;
        }
        app.ui.home = true;
    } else {
        app.show_error("Esta página não existe no DesignFlow.".into());
    }
    app.ui.home_view.designflow = state;
}
pub fn current_path(app: &FigApp) -> String {
    let state = &app.ui.home_view.designflow;
    if !app.ui.home && app.has_files() {
        if let Some(doc) = app.session.docs.get(app.session.active) {
            if let Some(project) = state.store.as_ref().and_then(|store| {
                store.data.projects.iter().find(|p| store.project_path(&p.id).ok().is_some_and(|path| doc.path.as_deref() == path.to_str()))
            }) {
                return format!("/editor/{}", project.id);
            }
            return format!("/editor/{}", doc.uid);
        }
    }
    state.route.path().into()
}

/// Called after the actual canvas has supplied its current viewport dimensions.
pub fn fit_pending(app: &mut FigApp, ctx: &egui::Context) {
    if !std::mem::take(&mut app.ui.home_view.designflow.fit_pending) {
        return;
    }
    let _ = app.run("view.zoomFit", json!({}));
    let view = app.session.ds().ok().map(|document| document.view.clone());
    if let Some(view) = view {
        let _ = app.run("view.set", json!({"zoom": view.zoom.min(0.64), "cx": view.cx, "cy": view.cy}));
    }
    ctx.request_repaint();
}

pub(super) fn show(app: &mut FigApp, root: &mut Ui) {
    let ctx = root.ctx().clone();
    let mut state = std::mem::take(&mut app.ui.home_view.designflow);
    let account = app.auth.as_ref().and_then(crate::auth::Auth::account_email).unwrap_or_default().to_owned();
    if state.account != account {
        state = State { account, ..State::default() };
    }
    if !state.loading_started {
        state.load(&ctx);
    }
    state.poll(app);
    if app.ui.home_view.section == super::Section::Templates {
        state.go(Route::Templates);
        app.ui.home_view.section = super::Section::Recents;
    }
    if state.refresh_needed && state.pending.is_none() {
        state.refresh_needed = false;
        if let Some(store) = &mut state.store {
            let updates: Vec<_> = store
                .data
                .projects
                .iter()
                .filter_map(|project| {
                    let path = canonical_path(&store.project_path(&project.id).ok()?);
                    let document = app
                        .session
                        .docs
                        .iter()
                        .find(|document| document.path.as_deref().is_some_and(|value| canonical_path(Path::new(value)) == path))?;
                    (document.doc.name != project.title).then(|| (project.id.clone(), document.doc.name.clone()))
                })
                .collect();
            for (id, name) in updates {
                if let Err(error) = store.set_project_title(&id, &name) {
                    state.error = Some(error);
                    break;
                }
            }
            if let Err(error) = store.refresh() {
                state.error = Some(error);
            }
        }
    }
    if app.ui.home || !app.has_files() {
        let t = Tokens::get(&ctx);
        let screen = root.max_rect();
        let mobile = screen.width() < metrics::COMPACT;
        let rail = screen.width() < metrics::TABLET;
        if app.ui.title_bar != crate::TitleBar::Native {
            egui::Panel::top("designflow_hub_caption_space").exact_size(24.0).frame(egui::Frame::NONE.fill(t.bg)).show(root, |_| {});
        }
        let side = if mobile {
            0.0
        } else if rail {
            80.0
        } else {
            metrics::SIDEBAR
        };
        if side > 0.0 {
            egui::Panel::left("designflow_hub_sidebar")
                .exact_size(side)
                .resizable(false)
                .frame(egui::Frame::NONE.fill(t.panel).inner_margin(Margin::symmetric(if rail { 12 } else { 20 }, 24)))
                .show(root, |ui| sidebar(ui, &mut state, &t, rail));
        }
        egui::Panel::top("designflow_hub_topbar")
            .exact_size(if mobile { 64.0 } else { metrics::TOPBAR })
            .frame(egui::Frame::NONE.fill(t.bg).inner_margin(Margin::symmetric(if mobile { 16 } else { 40 }, if mobile { 10 } else { 20 })))
            .show(root, |ui| topbar(ui, &mut state, &t, mobile));
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.bg)).show(root, |ui| {
            paint_hub_background(ui.painter(), ui.max_rect(), &t);
            ui.style_mut().spacing.item_spacing = vec2(12.0, 12.0);
            ui.style_mut().spacing.button_padding = vec2(16.0, 10.0);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            let padding = if mobile {
                16.0
            } else if screen.width() < metrics::TABLET {
                24.0
            } else {
                metrics::PADDING
            };
            egui::ScrollArea::vertical().id_salt(("hub", state.route.path())).auto_shrink([false, false]).show(ui, |ui| {
                egui::Frame::NONE.inner_margin(Margin::same(padding as i8)).show(ui, |ui| {
                    let content_width = ui.available_width().min(metrics::MAX_WIDTH);
                    ui.set_width(content_width);
                    if let Some(error) = state.error.clone() {
                        w::status(ui, &error, true, &t);
                        ui.horizontal_wrapped(|ui| {
                            if w::button(ui, "Recarregar", false, state.pending.is_none(), &t).clicked() {
                                state.load(ui.ctx());
                            }
                            if w::button(ui, "Fechar aviso", false, true, &t).clicked() {
                                state.error = None;
                            }
                        });
                        ui.add_space(16.0);
                    }
                    if let Some(feedback) = &state.feedback {
                        w::status(ui, feedback, false, &t);
                        ui.add_space(12.0);
                    }
                    if let Some(pending) = &state.pending {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            w::status(ui, pending.label, false, &t);
                        });
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
                    }
                    if state.store.is_some() {
                        ui.add_enabled_ui(state.pending.is_none(), |ui| body(app, ui, &mut state, &t));
                    } else if state.pending.is_none() {
                        w::status(ui, "Não foi possível abrir seu espaço local. Use Recarregar para tentar novamente.", true, &t);
                    }
                });
            });
        });
        if mobile && state.drawer {
            let id = Id::unique("designflow_mobile_drawer");
            let area = egui::Modal::default_area(id).anchor(Align2::LEFT_TOP, vec2(0.0, 0.0));
            let modal = egui::Modal::new(id).area(area).frame(egui::Frame::NONE.fill(t.panel).inner_margin(Margin::same(20))).show(&ctx, |ui| {
                ui.set_width(240.0);
                ui.set_min_height((screen.height() - 40.0).max(200.0));
                sidebar(ui, &mut state, &t, false);
            });
            if modal.should_close() || !state.drawer {
                state.drawer = false;
                if let Some(opener) = state.drawer_opener.take() {
                    ctx.memory_mut(|m| m.request_focus(opener));
                }
            }
        }
        modal(app, &mut state, &ctx, &t);
        handle_drops(&mut state, &ctx);
    }
    app.ui.home_view.designflow = if app.auth.as_ref().is_some_and(|auth| !auth.unlocked()) { State::default() } else { state };
}

fn sidebar(ui: &mut Ui, state: &mut State, t: &Tokens, rail: bool) {
    ui.set_width(if rail { 56.0 } else { metrics::SIDEBAR - 40.0 });
    let brand = ui.horizontal(|ui| {
        let (r, response) = ui.allocate_exact_size(vec2(32.0, 36.0), Sense::click());
        theme::paint_app_icon(ui, r.shrink(3.0));
        a11y::button(&response, "Início");
        if response.clicked() {
            state.go(Route::Home);
        }
        if !rail {
            ui.label(RichText::new("DesignFlow").font(theme::designflow_semibold(19.0)).color(t.text));
        }
    });
    let _ = brand;
    ui.add_space(38.0);
    for route in [Route::Home, Route::Projects, Route::Resources, Route::Library] {
        nav_item(ui, state, route, t, rail);
        ui.add_space(6.0);
    }
    ui.add_space(14.0);
    ui.separator();
    ui.add_space(14.0);
    nav_item(ui, state, Route::Trash, t, rail);
    let bottom = ui.available_height() - 120.0;
    if bottom > 24.0 {
        ui.add_space(bottom);
    }
    for route in [Route::Settings, Route::Account] {
        nav_item(ui, state, route, t, rail);
        ui.add_space(6.0);
    }
}
fn nav_item(ui: &mut Ui, state: &mut State, route: Route, t: &Tokens, rail: bool) {
    let active = state.route == route;
    let (r, response) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
    a11y::tab(&response, route.label(), active);
    if response.hovered() || active {
        ui.painter().rect_filled(r, 12, if active { t.selected_row } else { t.hover });
    }
    let icon_rect = Rect::from_center_size(pos2(r.left() + if rail { r.width() * 0.5 } else { 24.0 }, r.center().y), vec2(20.0, 20.0));
    let icon_color = if active { t.accent } else { t.icon };
    if route == Route::Projects {
        paint_folder_icon(ui.painter(), icon_rect, icon_color);
    } else {
        paint_hub_icon(ui.painter(), icon_rect, if route == Route::Library { Icon::Grid } else { route.icon() }, icon_color);
    }
    if !rail {
        ui.painter().text(
            pos2(r.left() + 50.0, r.center().y),
            Align2::LEFT_CENTER,
            route.label(),
            theme::designflow_regular(15.0),
            if active { t.accent_text } else { t.text },
        );
    }
    if response.has_focus() {
        theme::paint_focus_ring(ui, r, 12, t);
    }
    if response.clicked() {
        state.go(route);
    }
    response.on_hover_text(route.label());
}
fn topbar(ui: &mut Ui, state: &mut State, t: &Tokens, mobile: bool) {
    ui.horizontal(|ui| {
        if mobile {
            let response = w::icon_button(ui, Icon::Menu, "Abrir navegação", state.drawer, t);
            if response.clicked() {
                state.drawer = true;
                state.drawer_opener = Some(response.id);
            }
        }
        let searchable = !matches!(state.route, Route::Account | Route::Settings | Route::Import);
        let width = (ui.available_width() - 64.0).clamp(60.0, 540.0);
        let hint = match state.route {
            Route::Templates => "Buscar modelos…",
            Route::Library => "Buscar na biblioteca…",
            Route::Resources => "Buscar recursos…",
            Route::Trash => "Buscar na lixeira…",
            _ => "Buscar projetos…",
        };
        let field =
            egui::Frame::new().fill(t.panel).stroke(Stroke::new(1.0, t.border)).corner_radius(12).inner_margin(Margin::symmetric(12, 0)).show(
                ui,
                |ui| {
                    ui.set_width((width - 26.0).max(1.0));
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let (icon, _) = ui.allocate_exact_size(vec2(20.0, 44.0), Sense::hover());
                        paint_hub_icon(ui.painter(), Rect::from_center_size(icon.center(), vec2(20.0, 20.0)), Icon::Search, t.text_secondary);
                        let shortcut = if mobile { 0.0 } else { 64.0 };
                        let input = ui.add_enabled(
                            searchable,
                            egui::TextEdit::singleline(&mut state.query)
                                .id_salt("hub-search")
                                .hint_text(RichText::new(hint).font(theme::designflow_regular(15.0)))
                                .font(theme::designflow_regular(15.0))
                                .desired_width((width - 54.0 - shortcut).max(24.0))
                                .frame(egui::Frame::NONE)
                                .margin(Margin::symmetric(0, 12)),
                        );
                        if !mobile {
                            let (badge, _) = ui.allocate_exact_size(vec2(56.0, 26.0), Sense::hover());
                            ui.painter().rect_filled(badge, 6, t.field);
                            ui.painter().text(badge.center(), Align2::CENTER_CENTER, "Ctrl K", theme::designflow_regular(13.0), t.text_secondary);
                        }
                        input
                    })
                    .inner
                },
            );
        let response = field.inner;
        if response.has_focus() {
            theme::paint_focus_ring(ui, field.response.rect, 12, t);
        }
        a11y::text(&response, "Buscar nesta página", &state.query);
        if searchable && ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) {
            response.request_focus();
        }
        if response.has_focus() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            state.query.clear();
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (rect, response) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
            a11y::button(&response, "Minha conta");
            ui.painter().circle_filled(rect.center(), 22.0, t.selected_row);
            let name = state.store.as_ref().map(|store| store.data.profile.name.trim()).filter(|name| !name.is_empty()).unwrap_or(&state.account);
            if let Some(initial) = name.chars().next() {
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    initial.to_uppercase().to_string(),
                    theme::designflow_semibold(16.0),
                    t.accent_text,
                );
            } else {
                paint_hub_icon(ui.painter(), rect.shrink(12.0), Icon::User, t.accent_text);
            }
            if response.has_focus() {
                theme::paint_focus_ring(ui, rect, 22, t);
            }
            if response.clicked() {
                state.go(Route::Account);
            }
        });
    });
}

fn body(app: &mut FigApp, ui: &mut Ui, state: &mut State, t: &Tokens) {
    match state.route {
        Route::Home => home(app, ui, state, t),
        Route::Projects => projects(app, ui, state, t),
        Route::Templates => templates(ui, state, t),
        Route::Import => import(app, ui, state, t),
        Route::Resources | Route::Library | Route::Trash => {
            state.collections.query = state.query.clone();
            let Some(store) = &mut state.store else { return };
            let action = match state.route {
                Route::Resources => collections::resources(app, ui, store, &mut state.collections, t),
                Route::Library => collections::library(app, ui, store, &mut state.collections, t),
                _ => collections::trash(app, ui, store, &mut state.collections, t),
            };
            match action {
                Some(collections::Action::CreateFromLibrary(id)) => state.job(ui.ctx(), "Criando projeto local…", move |store| {
                    store.create_project_from_library(&id).map(|project| Completed::Project(project.id))
                }),
                Some(collections::Action::Import) => state.go(Route::Import),
                None => {}
            }
        }
        Route::Account => {
            w::title(ui, "Minha conta", "Seu perfil local do DesignFlow.", t);
            if let Some(store) = &mut state.store {
                account_settings::show_account(app, ui, store, &mut state.settings, t);
            }
        }
        Route::Settings => {
            w::title(ui, "Configurações", "Preferências do seu espaço e do editor.", t);
            if let Some(store) = &mut state.store {
                account_settings::show_settings(app, ui, store, &mut state.settings, t);
            }
        }
    }
}
fn home(app: &mut FigApp, ui: &mut Ui, state: &mut State, t: &Tokens) {
    w::title(ui, "Bem-vindo ao DesignFlow", "Crie, organize e desenvolva seus projetos em um fluxo simples e profissional.", t);
    if !state.query.trim().is_empty() {
        projects_list(app, ui, state, t);
        return;
    }
    let count = state.store.as_ref().map_or(0, |s| s.data.projects.len());
    ui.add_space((ui.available_height() * 0.08).clamp(24.0, 64.0));
    ui.vertical_centered(|ui| {
        ui.set_max_width(ui.available_width().min(620.0));
        let illustration_width = ui.available_width().min(if ui.available_width() < 480.0 { 220.0 } else { 280.0 });
        let (r, _) = ui.allocate_exact_size(vec2(illustration_width, illustration_width * 0.82), Sense::hover());
        paint_empty_illustration(ui.painter(), r, t);
        ui.add_space(20.0);
        ui.label(
            RichText::new(if count == 0 { "Crie seu primeiro projeto" } else { "Continue seu próximo projeto" })
                .font(theme::designflow_semibold(if ui.available_width() < 480.0 { 24.0 } else { 30.0 }))
                .color(t.text),
        );
        ui.add_space(10.0);
        w::muted(ui, "Comece do zero, use um modelo ou importe um arquivo.", t);
        ui.add_space(26.0);
        let create = action_button(ui, "Novo projeto", Icon::Plus, true, 200.0, t);
        if create.clicked() {
            state.name = "Sem título".into();
            state.open_modal(ui, Modal::Create);
        }
        ui.add_space(18.0);
        let row_width = 3.0 * 160.0 + 2.0 * 16.0;
        let actions = |ui: &mut Ui| {
            ui.spacing_mut().item_spacing = vec2(16.0, 12.0);
            if action_button(ui, "Em branco", Icon::Page, false, 160.0, t).clicked() {
                state.name = "Sem título".into();
                state.open_modal(ui, Modal::Create);
            }
            if action_button(ui, "Modelo", Icon::Grid, false, 160.0, t).clicked() {
                state.go(Route::Templates);
            }
            if action_button(ui, "Importar", Icon::Import, false, 160.0, t).clicked() {
                state.go(Route::Import);
            }
        };
        if ui.available_width() < row_width {
            ui.vertical_centered(actions);
        } else {
            // The parent centers this exactly measured group as one widget.
            ui.allocate_ui_with_layout(vec2(row_width, 48.0), Layout::left_to_right(Align::Center), actions);
        }
        if count > 0 {
            ui.add_space(28.0);
            if w::button(ui, "Ver meus projetos", false, true, t).clicked() {
                state.go(Route::Projects);
            }
        }
    });
}
fn projects(app: &mut FigApp, ui: &mut Ui, state: &mut State, t: &Tokens) {
    let narrow = ui.available_width() < 650.0;
    if narrow {
        w::title(ui, "Seus projetos", "Gerencie, edite e organize todos os seus projetos em um só lugar.", t);
    }
    ui.horizontal_top(|ui| {
        if !narrow {
            let width = (ui.available_width() - 224.0).max(100.0);
            ui.vertical(|ui| {
                ui.set_width(width);
                w::title(ui, "Seus projetos", "Gerencie, edite e organize todos os seus projetos em um só lugar.", t);
            });
        }
        if action_button(ui, "Novo projeto", Icon::Plus, true, 200.0, t).clicked() {
            state.name = "Sem título".into();
            state.open_modal(ui, Modal::Create);
        }
    });
    if narrow {
        ui.add_space(22.0);
    }
    ui.horizontal_wrapped(|ui| {
        for (i, label) in ["Recentes", "Em andamento", "Favoritos"].into_iter().enumerate() {
            if w::chip(ui, label, state.project_filter == i, t) {
                state.project_filter = i;
            }
        }
        ui.separator();
        if w::chip(ui, "Grade", !state.list, t) {
            state.list = false;
        }
        if w::chip(ui, "Lista", state.list, t) {
            state.list = true;
        }
        craft_ui::ComboBox::from_id_salt("hub-project-sort").selected_text(if state.sort == 0 { "Mais recentes" } else { "Nome" }).show_ui(
            ui,
            |ui| {
                ui.selectable_value(&mut state.sort, 0, "Mais recentes");
                ui.selectable_value(&mut state.sort, 1, "Nome");
            },
        );
    });
    ui.add_space(22.0);
    projects_list(app, ui, state, t);
}
fn projects_list(app: &mut FigApp, ui: &mut Ui, state: &mut State, t: &Tokens) {
    let query = state.query.trim().to_lowercase();
    let mut items: Vec<_> = state
        .store
        .as_ref()
        .map(|s| {
            s.data
                .projects
                .iter()
                .filter(|p| {
                    p.title.to_lowercase().contains(&query)
                        && (state.project_filter != 1 || p.status == "active")
                        && (state.project_filter != 2 || p.favorite)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    if state.sort == 0 {
        items.sort_by_key(|p| std::cmp::Reverse(p.updated_at));
    } else {
        items.sort_by_key(|p| p.title.to_lowercase());
    }
    if items.is_empty() {
        w::muted(
            ui,
            if query.is_empty() && state.project_filter == 0 {
                "Seus projetos vão aparecer aqui depois que você criar ou importar um arquivo."
            } else {
                "Nenhum projeto corresponde aos filtros e à busca."
            },
            t,
        );
        return;
    }
    let columns = if state.list { 1 } else { ((ui.available_width() + 22.0) / 300.0).floor().clamp(1.0, 4.0) as usize };
    for row in items.chunks(columns) {
        ui.columns(columns, |uis| {
            for (index, item) in row.iter().enumerate() {
                let ui = &mut uis[index];
                w::card(ui, t, |ui| {
                    let path = state.store.as_ref().and_then(|s| s.project_path(&item.id).ok());
                    if !state.list {
                        if let Some(path) = path.as_deref() {
                            let preview_width = ui.available_width();
                            collections::document_preview(
                                ui,
                                &mut state.collections,
                                path,
                                item.updated_at,
                                vec2(preview_width, (preview_width / 1.6).clamp(130.0, 194.0)),
                                t,
                            );
                        }
                        ui.add_space(14.0);
                    }
                    let open = ui.add(
                        egui::Button::new(RichText::new(&item.title).font(theme::designflow_semibold(20.0)))
                            .frame(false)
                            .min_size(vec2(ui.available_width(), 44.0)),
                    );
                    a11y::button(&open, &format!("Abrir {}", item.title));
                    if open.clicked() {
                        state.open_project(app, &item.id);
                    }
                    w::muted(ui, &format!("Atualizado {}", date(item.updated_at)), t);
                    ui.horizontal_wrapped(|ui| {
                        if w::icon_button(ui, Icon::Star, if item.favorite { "Remover favorito" } else { "Adicionar favorito" }, item.favorite, t)
                            .clicked()
                        {
                            if let Some(store) = &mut state.store {
                                if let Err(error) = store.favorite_project(&item.id, !item.favorite) {
                                    state.error = Some(error);
                                }
                            }
                        }
                        if w::icon_button(ui, Icon::Trash, "Mover para a lixeira", false, t).clicked() {
                            state.open_modal(ui, Modal::Trash(item.id.clone()));
                        }
                    });
                });
            }
        });
        ui.add_space(metrics::GAP);
    }
}

fn template_name(id: &str) -> &str {
    match id {
        "mobile-app" => "Aplicativo móvel",
        "landing-page" => "Página web",
        "dashboard" => "Painel de dados",
        "design-system" => "Identidade visual",
        "wireframe-kit" => "Kit de wireframes",
        "social-posts" => "Posts para redes sociais",
        "slides" => "Apresentação",
        "resume" => "Currículo",
        "moodboard" => "Moodboard",
        "icon-set" => "Conjunto de ícones",
        _ => id,
    }
}
fn template_category(id: &str) -> usize {
    match id {
        "social-posts" => 1,
        "slides" => 2,
        "resume" => 4,
        "moodboard" => 5,
        "mobile-app" | "landing-page" | "dashboard" | "wireframe-kit" => 3,
        "design-system" | "icon-set" => 6,
        _ => 0,
    }
}
fn template_summary(id: &str) -> &'static str {
    match id {
        "social-posts" => "Templates para Instagram, LinkedIn e muito mais.",
        "slides" => "Slides modernos e impactantes.",
        "landing-page" => "Modelos de páginas que convertem.",
        "resume" => "Currículos modernos e profissionais.",
        "moodboard" => "Organize referências e ideias visuais.",
        "design-system" => "Crie sua marca com consistência e estilo.",
        _ => "Conteúdo editável incluído no DesignFlow.",
    }
}
fn templates(ui: &mut Ui, state: &mut State, t: &Tokens) {
    w::title(ui, "Escolha um modelo", "Comece mais rápido com modelos prontos e personalizáveis.", t);
    ui.horizontal_wrapped(|ui| {
        for (i, label) in
            ["Todos", "Redes sociais", "Apresentações", "Páginas web", "Currículos", "Moodboard", "Identidade visual"].into_iter().enumerate()
        {
            let icon = [Icon::Grid, Icon::Image, Icon::Panels, Icon::Page, Icon::Page, Icon::Grid, Icon::Component][i];
            let width = ui.painter().layout_no_wrap(label.into(), theme::designflow_regular(15.0), t.text).size().x + 60.0;
            if action_button(ui, label, icon, state.template_filter == i, width, t).clicked() {
                state.template_filter = i;
            }
        }
    });
    ui.add_space(24.0);
    let query = state.query.trim().to_lowercase();
    let items: Vec<_> = ["social-posts", "slides", "landing-page", "resume", "moodboard", "design-system"]
        .into_iter()
        .filter_map(designflow_engine::templates::find)
        .filter(|template| {
            (state.template_filter == 0 || template_category(template.id) == state.template_filter)
                && (template_name(template.id).to_lowercase().contains(&query) || template.description.to_lowercase().contains(&query))
        })
        .collect();
    if items.is_empty() {
        w::muted(ui, "Nenhum modelo disponível nesta categoria ou busca.", t);
        return;
    }
    let cols = ((ui.available_width() + 22.0) / 330.0).floor().clamp(1.0, 3.0) as usize;
    for row in items.chunks(cols) {
        ui.columns(cols, |uis| {
            for (index, template) in row.iter().enumerate() {
                let ui = &mut uis[index];
                egui::Frame::new().fill(t.panel).stroke(Stroke::new(1.0, t.border)).corner_radius(16).show(ui, |ui| {
                    let width = ui.available_width();
                    let preview = ui
                        .scope_builder(egui::UiBuilder::new().sense(Sense::click()), |ui| {
                            template_preview(ui, template, width, width * 0.52, t);
                        })
                        .response;
                    a11y::button(&preview, &format!("Ver modelo {}", template_name(template.id)));
                    let mut open = preview.clicked();
                    egui::Frame::NONE.inner_margin(20).show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width((width - 96.0).max(80.0));
                                ui.label(RichText::new(template_name(template.id)).font(theme::designflow_semibold(18.0)));
                                ui.label(RichText::new(template_summary(template.id)).font(theme::designflow_regular(14.0)).color(t.text_secondary));
                            });
                            open |= w::icon_button(ui, Icon::Arrow, "Ver modelo", true, t).clicked();
                        });
                    });
                    if open {
                        state.name = template_name(template.id).into();
                        state.open_modal(ui, Modal::Template(template.id));
                    }
                });
            }
        });
        ui.add_space(22.0);
    }
}
fn template_preview(ui: &mut Ui, template: &designflow_engine::templates::Template, width: f32, height: f32, t: &Tokens) {
    let id = Id::unique(("hub-template", template.id));
    let cached = ui.ctx().data(|d| d.get_temp::<Option<super::Thumb>>(id));
    let picture = cached.unwrap_or_else(|| {
        let picture = super::thumbnail(ui.ctx(), template, designflow_color::Color::WHITE);
        ui.ctx().data_mut(|d| d.insert_temp(id, picture.clone()));
        picture
    });
    let (r, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    ui.painter().rect_filled(r, 8, t.field);
    if let Some(picture) = picture {
        let size = picture.texture.size_vec2();
        let scale = (r.width() / size.x).min(r.height() / size.y);
        let fit = Rect::from_center_size(r.center(), size * scale);
        ui.painter().image(picture.texture.id(), fit, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    } else {
        icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(28.0, 28.0)), Icon::Frame, t.icon_secondary);
    }
}
fn import(app: &mut FigApp, ui: &mut Ui, state: &mut State, t: &Tokens) {
    w::title(ui, "Importar", "Adicione arquivos ao DesignFlow para usar em seus projetos.", t);
    let width = ui.available_width().min(960.0);
    let inset = ((ui.available_width() - width) * 0.5).max(0.0);
    ui.horizontal_top(|ui| {
        ui.add_space(inset);
        ui.vertical(|ui| {
            ui.set_width(width);
            let dropzone = egui::Frame::new().fill(t.field.gamma_multiply(0.35)).corner_radius(24).inner_margin(24).show(ui, |ui| {
                ui.set_width((width - 48.0).max(1.0));
                ui.vertical_centered(|ui| {
                    let picture_width = ui.available_width().min(200.0);
                    let (r, _) = ui.allocate_exact_size(vec2(picture_width, picture_width * 0.82), Sense::hover());
                    paint_empty_illustration_with_icon(ui.painter(), r, t, Icon::Import);
                    ui.add_space(8.0);
                    ui.label(RichText::new("Arraste arquivos aqui").font(theme::designflow_semibold(26.0)));
                    w::muted(ui, "Ou selecione um arquivo do seu computador.", t);
                    ui.add_space(18.0);
                    if action_button(ui, "Selecionar arquivo", Icon::Page, true, 240.0, t).clicked() && state.pending.is_none() {
                        app.pick_open("Importar arquivo", workspace::IMPORT_EXTENSIONS, |app, path| {
                            let ctx = app.ui_context.clone();
                            if let Some(ctx) = ctx {
                                let mut state = std::mem::take(&mut app.ui.home_view.designflow);
                                schedule_import(&mut state, &ctx, path);
                                app.ui.home_view.designflow = state;
                            }
                        });
                    }
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new("Formatos suportados: DesignFlow, PNG, JPG, SVG e WebP. Até 256 MiB por arquivo.")
                            .font(theme::designflow_regular(13.0))
                            .color(t.text_secondary),
                    );
                    ui.add_space(4.0);
                });
            });
            let r = dropzone.response.rect.shrink(1.0);
            for points in [
                [r.left_top() + vec2(20.0, 0.0), r.right_top() - vec2(20.0, 0.0)],
                [r.left_bottom() + vec2(20.0, 0.0), r.right_bottom() - vec2(20.0, 0.0)],
                [r.left_top() + vec2(0.0, 20.0), r.left_bottom() - vec2(0.0, 20.0)],
                [r.right_top() + vec2(0.0, 20.0), r.right_bottom() - vec2(0.0, 20.0)],
            ] {
                ui.painter().extend(egui::Shape::dashed_line(&points, Stroke::new(1.0, t.accent.gamma_multiply(0.28)), 5.0, 3.0));
            }
            let imports: Vec<_> = state
                .store
                .as_ref()
                .map(|s| s.data.library.iter().filter(|i| matches!(i.kind.as_str(), "upload" | "image")).take(4).cloned().collect())
                .unwrap_or_default();
            if !imports.is_empty() {
                ui.add_space(32.0);
                ui.label(RichText::new("Importações recentes").font(theme::designflow_semibold(20.0)));
                ui.add_space(12.0);
                w::card(ui, t, |ui| {
                    for item in imports {
                        ui.horizontal(|ui| {
                            let (tile, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
                            ui.painter().rect_filled(tile, 8, t.selected_row);
                            paint_hub_icon(ui.painter(), tile.shrink(12.0), if item.kind == "image" { Icon::Image } else { Icon::Page }, t.accent);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(item.title).font(theme::designflow_semibold(14.0)));
                                ui.label(RichText::new(date(item.created_at)).font(theme::designflow_regular(13.0)).color(t.text_secondary));
                            });
                        });
                        ui.add_space(6.0);
                    }
                });
            }
        });
    });
}
fn schedule_import(state: &mut State, ctx: &egui::Context, path: PathBuf) {
    state.job(ctx, "Validando e importando o arquivo…", move |store| {
        let result = store.import_file(&path)?;
        Ok(Completed::Imported(result.project.id, result.warnings))
    });
}
fn handle_drops(state: &mut State, ctx: &egui::Context) {
    if state.route != Route::Import || state.pending.is_some() {
        return;
    }
    let dropped = ctx.input(|i| i.raw.dropped_files.clone());
    if dropped.is_empty() {
        return;
    }
    if dropped.len() != 1 {
        state.error = Some("Selecione apenas um arquivo por importação.".into());
        return;
    }
    let path = dropped[0].path();
    if !path.as_os_str().is_empty() {
        schedule_import(state, ctx, path.to_owned());
    } else {
        state.error = Some("Este arquivo não oferece um caminho local legível.".into());
    }
}
fn modal(app: &mut FigApp, state: &mut State, ctx: &egui::Context, t: &Tokens) {
    let kind = match &state.modal {
        Some(Modal::Create) => 0,
        Some(Modal::Template(_)) => 1,
        Some(Modal::Trash(_)) => 2,
        None => return,
    };
    let mut close = false;
    let mut submit = false;
    let mut create_template = None;
    let mut trash_id = None;
    let busy = state.pending.is_some();
    let response = egui::Modal::new(Id::unique("hub-operation-modal"))
        .frame(egui::Frame::new().fill(t.panel).stroke(Stroke::new(1.0, t.border)).corner_radius(16).inner_margin(Margin::same(24)))
        .show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 80.0).clamp(200.0, 560.0));
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            let max_height = (ctx.content_rect().height() - 100.0).max(180.0);
            let content_height: f32 = if kind == 1 {
                530.0
            } else if kind == 0 {
                210.0
            } else {
                180.0
            };
            egui::ScrollArea::vertical()
                .id_salt(("hub-dialog-body", kind))
                .max_height(max_height)
                .min_scrolled_height(content_height.min(max_height))
                .show(ui, |ui| {
                    let title = match kind {
                        0 => "Novo projeto",
                        1 => "Prévia do modelo",
                        _ => "Mover projeto para a lixeira?",
                    };
                    ui.label(RichText::new(title).font(theme::designflow_semibold(22.0)));
                    ui.add_space(18.0);
                    if kind < 2 {
                        if let Some(Modal::Template(id)) = &state.modal {
                            if let Some(template) = designflow_engine::templates::find(id) {
                                template_preview(ui, template, ui.available_width(), 190.0, t);
                                ui.add_space(12.0);
                                w::muted(ui, &format!("{} · {}", template_name(id), template.size), t);
                                w::muted(ui, "Conteúdo de exemplo editável, incluído no aplicativo.", t);
                                create_template = Some((*id).to_owned());
                            }
                        }
                        ui.add_space(16.0);
                        ui.label("Nome do projeto");
                        let field = ui.add_sized(
                            vec2(ui.available_width(), 44.0),
                            egui::TextEdit::singleline(&mut state.name)
                                .font(theme::designflow_regular(15.0))
                                .char_limit(120)
                                .margin(Margin::same(12)),
                        );
                        a11y::text(&field, "Nome do projeto", &state.name);
                        if state.modal_focus {
                            field.request_focus();
                            state.modal_focus = false;
                        }
                        if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            submit = true;
                        }
                    } else if let Some(Modal::Trash(id)) = &state.modal {
                        trash_id = Some(id.clone());
                        w::muted(ui, "O projeto continuará disponível na Lixeira e poderá ser restaurado.", t);
                    }
                    if let Some(error) = &state.error {
                        w::status(ui, error, true, t);
                    }
                    ui.add_space(22.0);
                    ui.horizontal_wrapped(|ui| {
                        if w::button(ui, "Cancelar", false, !busy, t).clicked() {
                            close = true;
                        }
                        if w::button(
                            ui,
                            if kind == 2 {
                                "Mover para lixeira"
                            } else if kind == 1 {
                                "Usar modelo"
                            } else {
                                "Criar projeto"
                            },
                            true,
                            !busy && (kind == 2 || !state.name.trim().is_empty()),
                            t,
                        )
                        .clicked()
                        {
                            submit = true;
                        }
                    });
                });
        });
    if (close || response.should_close()) && !busy {
        state.close_modal(Some(ctx));
        return;
    }
    if submit && !busy {
        if let Some(id) = trash_id {
            let path = match state.store.as_ref().ok_or("Espaço local não disponível.".to_owned()).and_then(|store| store.project_path(&id)) {
                Ok(path) => canonical_path(&path),
                Err(error) => {
                    state.error = Some(error);
                    return;
                }
            };
            let open: Vec<_> = app
                .session
                .docs
                .iter()
                .filter(|document| document.path.as_deref().is_some_and(|value| canonical_path(Path::new(value)) == path))
                .map(|document| (document.uid, document.is_dirty()))
                .collect();
            if open.iter().any(|(_, dirty)| *dirty) {
                state.error = Some("Salve as alterações deste projeto antes de movê-lo para a lixeira.".into());
                return;
            }
            // Main-tab closure can remove branch tabs too. Resolve stable UIDs after each
            // close so no shifted index can accidentally close another project.
            for (uid, _) in open.into_iter().rev() {
                if let Some(index) = app.session.docs.iter().position(|document| document.uid == uid) {
                    if app.run("file.close", json!({"index": index})).is_none() {
                        state.error = Some("Não foi possível fechar o projeto. Nenhum item foi movido; tente novamente.".into());
                        return;
                    }
                }
            }
            if let Some(store) = &mut state.store {
                match store.trash_project(&id) {
                    Ok(()) => {
                        app.session.prefs.recent_files.retain(|value| canonical_path(Path::new(value)) != path);
                        app.session.prefs_unsaved = true;
                        app.ui.home = true;
                        state.close_modal(Some(ctx));
                        state.feedback = Some("Projeto movido para a lixeira.".into());
                    }
                    Err(error) => state.error = Some(error),
                }
            }
        } else {
            let name = state.name.trim().to_owned();
            state.job(ctx, "Criando projeto local…", move |store| {
                store.create_project(&name, create_template.as_deref()).map(|p| Completed::Project(p.id))
            });
        }
    }
}
fn canonical_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

fn action_button(ui: &mut Ui, label: &str, icon: Icon, primary: bool, width: f32, t: &Tokens) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width.min(ui.available_width()).max(1.0), 48.0), Sense::click());
    a11y::button(&response, label);
    let enabled = ui.is_enabled();
    let fill = if !enabled {
        t.field
    } else if primary {
        if response.hovered() { t.accent_hover } else { t.accent }
    } else if response.hovered() {
        t.hover
    } else {
        t.panel
    };
    let color = if !enabled {
        t.text_tertiary
    } else if primary {
        t.accent_on
    } else {
        t.text
    };
    ui.painter().rect_filled(rect, 12, fill);
    if !primary {
        ui.painter().rect_stroke(rect, 12, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    }
    let font = if primary { theme::designflow_semibold(15.0) } else { theme::designflow_regular(15.0) };
    let text_width = ui.painter().layout_no_wrap(label.into(), font.clone(), color).size().x;
    let left = rect.center().x - (20.0 + 12.0 + text_width) * 0.5;
    paint_hub_icon(ui.painter(), Rect::from_center_size(pos2(left + 10.0, rect.center().y), vec2(20.0, 20.0)), icon, color);
    ui.painter().text(pos2(left + 32.0, rect.center().y), Align2::LEFT_CENTER, label, font, color);
    if response.has_focus() {
        theme::paint_focus_ring(ui, rect, 12, t);
    }
    response
}

fn paint_folder_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let p = |x: f32, y: f32| pos2(rect.left() + x * rect.width() / 20.0, rect.top() + y * rect.height() / 20.0);
    painter.add(egui::Shape::closed_line(
        vec![p(2.0, 5.0), p(7.0, 5.0), p(9.0, 7.0), p(18.0, 7.0), p(18.0, 17.0), p(2.0, 17.0)],
        Stroke::new(1.8, color),
    ));
}

fn paint_hub_background(painter: &egui::Painter, rect: Rect, t: &Tokens) {
    let mut mesh = egui::Mesh::default();
    for index in 0..=32 {
        let fraction = index as f32 / 32.0;
        let x = rect.left() + rect.width() * (0.48 + fraction * 0.52);
        let y = rect.bottom() - rect.height() * (0.25 * fraction * fraction * (3.0 - 2.0 * fraction));
        mesh.colored_vertex(pos2(x, y), t.accent.gamma_multiply(0.045));
        mesh.colored_vertex(pos2(x, rect.bottom()), t.accent.gamma_multiply(0.025));
        if index > 0 {
            let i = index as u32 * 2;
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// The hub uses the reference's restrained 20 px outline vocabulary.
fn paint_hub_icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let p = |x: f32, y: f32| pos2(rect.left() + x * rect.width() / 20.0, rect.top() + y * rect.height() / 20.0);
    let stroke = Stroke::new(1.8, color);
    let line = |points: &[(f32, f32)]| {
        painter.add(egui::Shape::line(points.iter().map(|(x, y)| p(*x, *y)).collect(), stroke));
    };
    match icon {
        Icon::Plus => {
            line(&[(3.0, 10.0), (17.0, 10.0)]);
            line(&[(10.0, 3.0), (10.0, 17.0)]);
        }
        Icon::Grid => {
            for (x, y) in [(2.0, 2.0), (12.0, 2.0), (2.0, 12.0), (12.0, 12.0)] {
                painter.rect_stroke(Rect::from_min_max(p(x, y), p(x + 6.0, y + 6.0)), 1, stroke, egui::StrokeKind::Inside);
            }
        }
        Icon::Page => {
            line(&[(4.0, 2.0), (12.0, 2.0), (17.0, 7.0), (17.0, 18.0), (4.0, 18.0), (4.0, 2.0)]);
            line(&[(12.0, 2.0), (12.0, 7.0), (17.0, 7.0)]);
        }
        Icon::Import => {
            line(&[(3.0, 12.0), (3.0, 17.0), (17.0, 17.0), (17.0, 12.0)]);
            line(&[(6.0, 7.0), (10.0, 3.0), (14.0, 7.0)]);
            line(&[(10.0, 3.0), (10.0, 13.0)]);
        }
        Icon::Search => {
            painter.circle_stroke(p(8.5, 8.5), rect.width() * 0.30, stroke);
            line(&[(13.0, 13.0), (18.0, 18.0)]);
        }
        Icon::Home => {
            line(&[(2.0, 9.0), (10.0, 2.0), (18.0, 9.0)]);
            line(&[(4.0, 8.0), (4.0, 18.0), (8.0, 18.0), (8.0, 12.0), (12.0, 12.0), (12.0, 18.0), (16.0, 18.0), (16.0, 8.0)]);
        }
        Icon::Layers => {
            line(&[(10.0, 2.0), (18.0, 7.0), (10.0, 12.0), (2.0, 7.0), (10.0, 2.0)]);
            line(&[(2.0, 11.0), (10.0, 16.0), (18.0, 11.0)]);
            line(&[(2.0, 15.0), (10.0, 20.0), (18.0, 15.0)]);
        }
        Icon::Trash => {
            line(&[(3.0, 5.0), (17.0, 5.0)]);
            line(&[(7.0, 5.0), (7.0, 2.0), (13.0, 2.0), (13.0, 5.0)]);
            line(&[(5.0, 5.0), (6.0, 18.0), (14.0, 18.0), (15.0, 5.0)]);
            line(&[(8.0, 8.0), (8.5, 15.0)]);
            line(&[(12.0, 8.0), (11.5, 15.0)]);
        }
        _ => icons::paint(painter, rect, icon, color),
    }
}

/// Three rounded, skewed cards are vector artwork, with the reference's soft purple fade.
fn paint_empty_illustration(painter: &egui::Painter, rect: Rect, t: &Tokens) {
    paint_empty_illustration_with_icon(painter, rect, t, Icon::Plus);
}
fn paint_empty_illustration_with_icon(painter: &egui::Painter, rect: Rect, t: &Tokens, icon: Icon) {
    let scale = rect.width() / 280.0;
    for (left, top, width, height, opacity) in [(14.0, 48.0, 158.0, 132.0, 0.11), (66.0, 75.0, 166.0, 138.0, 0.17), (108.0, 89.0, 166.0, 138.0, 0.34)]
    {
        let radius = 12.0;
        let mut points = Vec::with_capacity(28);
        for (cx, cy, start) in [
            (radius, radius, std::f32::consts::PI),
            (width - radius, radius, -std::f32::consts::FRAC_PI_2),
            (width - radius, height - radius, 0.0),
            (radius, height - radius, std::f32::consts::FRAC_PI_2),
        ] {
            for step in 0..7 {
                let angle = start + step as f32 / 6.0 * std::f32::consts::FRAC_PI_2;
                let x = cx + radius * angle.cos();
                let y = cy + radius * angle.sin();
                points.push((pos2(rect.left() + (left + x) * scale, rect.top() + (top + y - x * 0.27) * scale), y / height));
            }
        }
        let color = |fraction: f32| t.accent.gamma_multiply(opacity * (1.0 - fraction * 0.78));
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(pos2(rect.left() + (left + width * 0.5) * scale, rect.top() + (top + height * 0.5 - width * 0.135) * scale), color(0.5));
        for (point, fraction) in &points {
            mesh.colored_vertex(*point, color(*fraction));
        }
        for index in 0..points.len() {
            mesh.add_triangle(0, index as u32 + 1, (index + 1) as u32 % points.len() as u32 + 1);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
    let center = pos2(rect.left() + 191.0 * scale, rect.top() + 137.0 * scale);
    let white = Color32::WHITE.gamma_multiply(0.92);
    paint_hub_icon(painter, Rect::from_center_size(center, vec2(28.0, 28.0) * scale), icon, white);
}
fn date(seconds: u64) -> String {
    chrono::DateTime::from_timestamp(seconds as i64, 0)
        .map(|date| date.with_timezone(&chrono::Local).format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_else(|| "data indisponível".into())
}
