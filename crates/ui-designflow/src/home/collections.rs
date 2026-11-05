//! The local catalogue, the account's private library and its recoverable trash.
//! Resource previews are generated from bundled source documents; private previews read only
//! paths checked by `workspace::Store`. Nothing in these screens contacts an external service.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use egui::{Id, Response, RichText, Stroke, Ui, vec2};

use crate::FigApp;
use crate::icons::{self, Icon};
use crate::theme::{Tokens, designflow_regular as regular, designflow_semibold as semibold};
use crate::workspace;

#[derive(Default)]
pub(super) struct State {
    pub query: String,
    resource_filter: usize,
    library_filter: usize,
    trash_filter: usize,
    alphabetical: bool,
    selected: BTreeSet<String>,
    dialog: Option<Dialog>,
    feedback: Option<(String, bool)>,
    preview: Option<Result<egui::TextureHandle, String>>,
    preview_reader: Option<Receiver<Result<egui::ColorImage, String>>>,
    card_previews: BTreeMap<String, Result<egui::TextureHandle, String>>,
    card_reader: Option<(String, Receiver<Result<egui::ColorImage, String>>)>,
    focus_first: bool,
    opener: Option<Id>,
}

pub(super) enum Action {
    Import,
    /// The parent creates a new editable project; the source library file stays private.
    CreateFromLibrary(String),
}

#[derive(Clone)]
enum Dialog {
    Resource(&'static str),
    Library(String),
    Rename { id: String, name: String },
    TrashLibrary { id: String, title: String },
    Delete { ids: Vec<String>, titles: Vec<String> },
}

struct Resource {
    template: &'static str,
    title: &'static str,
    summary: &'static str,
    category: usize,
    icon: Icon,
}

// These four entries are real editable templates shipped under the repository's
// MIT OR Apache-2.0 license. No remote catalogue or illustrative inventory is implied.
const RESOURCES: &[Resource] = &[
    Resource {
        template: "icon-set",
        title: "Ícones vetoriais",
        summary: "Componentes de ícones em uma grade de 24 px, com guias e tamanhos.",
        category: 1,
        icon: Icon::Grid,
    },
    Resource {
        template: "design-system",
        title: "Sistema de design",
        summary: "Cores, tipografia, variáveis e componentes editáveis de interface.",
        category: 4,
        icon: Icon::Component,
    },
    Resource {
        template: "wireframe-kit",
        title: "Kit de wireframes",
        summary: "Componentes e telas em escala de cinza para estruturar suas ideias.",
        category: 4,
        icon: Icon::LayoutGrid,
    },
    Resource {
        template: "mobile-app",
        title: "Interface de aplicativo",
        summary: "Quatro telas editáveis com componentes e um fluxo de protótipo.",
        category: 4,
        icon: Icon::Panels,
    },
];

pub(super) fn resources(app: &mut FigApp, ui: &mut Ui, store: &mut workspace::Store, state: &mut State, t: &Tokens) -> Option<Action> {
    ui.spacing_mut().interact_size.y = 44.0;
    state.receive_card_preview(ui.ctx());
    heading(ui, "Recursos", "Kits incluídos no DesignFlow para começar com uma base editável.", t);
    filters(ui, &mut state.resource_filter, &["Todos", "Ícones", "Ilustrações", "Mockups", "Componentes"], t);
    ui.add_space(24.0);
    feedback(ui, store, state, t);
    let query = state.query.trim().to_lowercase();
    let entries: Vec<_> = RESOURCES
        .iter()
        .filter(|resource| {
            (state.resource_filter == 0 || state.resource_filter == resource.category)
                && format!("{} {}", resource.title, resource.summary).to_lowercase().contains(&query)
        })
        .collect();
    if entries.is_empty() {
        empty(
            ui,
            "Nenhum recurso nesta seleção",
            "Este catálogo contém somente os kits locais disponíveis. Tente outra categoria ou busca.",
            Icon::Assets,
            t,
        );
    } else {
        let columns = if ui.available_width() >= 960.0 { 2 } else { 1 };
        let width = card_width(ui.available_width(), columns);
        for row in entries.chunks(columns) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 22.0;
                for resource in row {
                    card(ui, width, t, |ui| {
                        resource_card(ui, state, resource, t);
                    });
                }
            });
            ui.add_space(22.0);
        }
    }
    dialogs(app, ui, store, state, t)
}

pub(super) fn library(app: &mut FigApp, ui: &mut Ui, store: &mut workspace::Store, state: &mut State, t: &Tokens) -> Option<Action> {
    ui.spacing_mut().interact_size.y = 44.0;
    state.receive_card_preview(ui.ctx());
    let import = if ui.available_width() >= 760.0 {
        let title_width = (ui.available_width() - 272.0).max(1.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2(title_width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                heading(ui, "Biblioteca", "Acesse e organize seus arquivos, elementos e coleções salvas.", t);
            });
            import_button(ui, t).clicked()
        })
        .inner
    } else {
        heading(ui, "Biblioteca", "Acesse e organize seus arquivos, elementos e coleções salvas.", t);
        let clicked = import_button(ui, t).clicked();
        ui.add_space(24.0);
        clicked
    };
    if import {
        return Some(Action::Import);
    }
    filters(ui, &mut state.library_filter, &["Todos", "Favoritos", "Uploads", "Logos", "Cores", "Imagens", "Coleções"], t);
    ui.add_space(16.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Organizar por").font(regular(13.0)).color(t.text_secondary));
        egui::ComboBox::from_id_salt("library-order").selected_text(if state.alphabetical { "Nome" } else { "Última alteração" }).show_ui(ui, |ui| {
            ui.selectable_value(&mut state.alphabetical, false, "Última alteração");
            ui.selectable_value(&mut state.alphabetical, true, "Nome");
        });
        ui.add_space(12.0);
        ui.label(RichText::new("Adicione arquivos pela tela Importar ou salve um kit em Recursos.").font(regular(13.0)).color(t.text_secondary));
    });
    ui.add_space(24.0);
    feedback(ui, store, state, t);
    let query = state.query.trim().to_lowercase();
    let mut items: Vec<_> = store
        .data
        .library
        .iter()
        .filter(|item| {
            let category = match state.library_filter {
                1 => item.favorite,
                2 => matches!(item.kind.as_str(), "upload" | "image"),
                3 => item.kind == "logo",
                4 => item.kind == "palette",
                5 => item.kind == "image",
                6 => item.kind == "collection",
                _ => true,
            };
            category && item.title.to_lowercase().contains(&query)
        })
        .cloned()
        .collect();
    if state.alphabetical {
        items.sort_by_key(|item| item.title.to_lowercase());
    } else {
        items.sort_by_key(|item| std::cmp::Reverse(item.updated_at));
    }
    if items.is_empty() {
        let (title, description) = if store.data.library.is_empty() {
            ("Sua biblioteca está vazia", "Importe um arquivo ou adicione um recurso local. Seus itens aparecerão aqui.")
        } else if matches!(state.library_filter, 3 | 4) {
            ("Nenhum item nesta categoria", "A classificação de logos e paletas ainda não está disponível. Seus arquivos continuam em Todos.")
        } else {
            ("Nenhum arquivo encontrado", "Tente outro filtro ou ajuste os termos da busca.")
        };
        empty(ui, title, description, Icon::Library, t);
    } else {
        let columns = columns(ui.available_width(), 3);
        let width = card_width(ui.available_width(), columns);
        for row in items.chunks(columns) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 22.0;
                for item in row {
                    ui.push_id(&item.id, |ui| {
                        library_card(ui, width, t, |ui| {
                            let source = store.library_path(&item.id).ok().map(PreviewSource::File);
                            let preview_rect = card_picture(
                                ui,
                                state,
                                format!("{}-{}", item.id, item.updated_at),
                                source,
                                if item.kind == "image" { Icon::Image } else { Icon::Library },
                                t,
                            );
                            let preview_button = ui.interact(preview_rect, Id::unique(("library-preview", &item.id)), egui::Sense::click());
                            crate::a11y::button(&preview_button, &format!("Visualizar {}", item.title));
                            if preview_button.has_focus() {
                                ui.painter().rect_stroke(preview_rect, 12, Stroke::new(2.0, t.focus), egui::StrokeKind::Inside);
                            }
                            if preview_button.clicked() {
                                state.open(Dialog::Library(item.id.clone()), preview_button.id);
                                match store.library_path(&item.id) {
                                    Ok(path) => state.start_preview(ui.ctx(), PreviewSource::File(path)),
                                    Err(error) => state.preview = Some(Err(error)),
                                }
                            }
                            ui.add_space(8.0);
                            egui::Frame::new().inner_margin(8).show(ui, |ui| {
                                ui.horizontal_top(|ui| {
                                    let text_width = (ui.available_width() - 104.0).max(80.0);
                                    ui.allocate_ui_with_layout(vec2(text_width, 44.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                                        ui.add(egui::Label::new(RichText::new(&item.title).font(semibold(18.0)).color(t.text)).wrap());
                                        ui.add(
                                            egui::Label::new(RichText::new(library_kind(&item.kind)).font(regular(13.0)).color(t.text_secondary))
                                                .wrap(),
                                        );
                                    });
                                    let favorite = icon_button(
                                        ui,
                                        Icon::Star,
                                        if item.favorite { "Remover dos favoritos" } else { "Adicionar aos favoritos" },
                                        item.favorite,
                                        t,
                                    );
                                    if favorite.clicked() {
                                        state.result(
                                            store.favorite_library(&item.id, !item.favorite),
                                            if item.favorite { "Removido dos favoritos." } else { "Adicionado aos favoritos." },
                                        );
                                    }
                                    let more = icon_button(ui, Icon::Dots, "Mais opções do arquivo", false, t);
                                    crate::right::menu_popup(&more).show(|ui| {
                                        if ui.button("Visualizar").clicked() {
                                            state.open(Dialog::Library(item.id.clone()), more.id);
                                            match store.library_path(&item.id) {
                                                Ok(path) => state.start_preview(ui.ctx(), PreviewSource::File(path)),
                                                Err(error) => state.preview = Some(Err(error)),
                                            }
                                            ui.close();
                                        }
                                        if ui.button("Renomear").clicked() {
                                            state.open(Dialog::Rename { id: item.id.clone(), name: item.title.clone() }, more.id);
                                            ui.close();
                                        }
                                        if ui.button("Mover para lixeira").clicked() {
                                            state.open(Dialog::TrashLibrary { id: item.id.clone(), title: item.title.clone() }, more.id);
                                            ui.close();
                                        }
                                    });
                                });
                            });
                        });
                    });
                }
            });
            ui.add_space(22.0);
        }
    }
    dialogs(app, ui, store, state, t)
}

pub(super) fn trash(app: &mut FigApp, ui: &mut Ui, store: &mut workspace::Store, state: &mut State, t: &Tokens) -> Option<Action> {
    ui.spacing_mut().interact_size.y = 44.0;
    state.selected.retain(|id| store.data.trash.iter().any(|item| &item.id == id));
    if ui.available_width() >= 1024.0 {
        let heading_width = ui.available_width() - 480.0;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(vec2(heading_width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                heading(ui, "Lixeira", "Itens removidos recentemente. Você pode restaurar ou excluir permanentemente.", t);
            });
            ui.vertical(|ui| {
                ui.add_space(6.0);
                trash_actions(ui, store, state, t);
            });
        });
    } else {
        heading(ui, "Lixeira", "Itens removidos recentemente. Você pode restaurar ou excluir permanentemente.", t);
        trash_actions(ui, store, state, t);
        ui.add_space(24.0);
    }
    filters(ui, &mut state.trash_filter, &["Todos", "Projetos", "Arquivos"], t);
    ui.add_space(24.0);
    feedback(ui, store, state, t);
    let query = state.query.trim().to_lowercase();
    let items: Vec<_> = store
        .data
        .trash
        .iter()
        .filter(|item| {
            (state.trash_filter == 0 || (state.trash_filter == 1 && item.kind == "project") || (state.trash_filter == 2 && item.kind == "file"))
                && item.title.to_lowercase().contains(&query)
        })
        .cloned()
        .collect();
    if items.is_empty() {
        empty(
            ui,
            if store.data.trash.is_empty() { "Sua lixeira está vazia" } else { "Nenhum item encontrado" },
            "Os itens movidos para cá podem ser restaurados antes da exclusão permanente.",
            Icon::Undo,
            t,
        );
    } else {
        let all_visible = items.iter().all(|item| state.selected.contains(&item.id));
        let mut checked = all_visible;
        if ui.checkbox(&mut checked, "Selecionar itens visíveis").changed() {
            for item in &items {
                if checked {
                    state.selected.insert(item.id.clone());
                } else {
                    state.selected.remove(&item.id);
                }
            }
        }
        ui.add_space(12.0);
        egui::Frame::new().fill(t.panel).stroke(Stroke::new(1.0, t.border)).corner_radius(16).show(ui, |ui| {
            let row_count = items.len();
            for (row_index, item) in items.into_iter().enumerate() {
                ui.push_id(&item.id, |ui| {
                    egui::Frame::new().inner_margin(16).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let compact = ui.available_width() < 560.0;
                        ui.horizontal(|ui| {
                            let mut checked = state.selected.contains(&item.id);
                            let select = ui.add_sized(vec2(28.0, 44.0), egui::Checkbox::new(&mut checked, ""));
                            crate::a11y::checkbox(&select, &format!("Selecionar {}", item.title), checked);
                            if select.changed() {
                                if checked {
                                    state.selected.insert(item.id.clone());
                                } else {
                                    state.selected.remove(&item.id);
                                }
                            }
                            if !compact {
                                let (thumbnail, _) = ui.allocate_exact_size(vec2(64.0, 56.0), egui::Sense::hover());
                                ui.painter().rect_filled(thumbnail, 10, t.selected_row);
                                icons::paint(
                                    ui.painter(),
                                    thumbnail.shrink2(vec2(20.0, 16.0)),
                                    if item.kind == "project" { Icon::Page } else { Icon::Image },
                                    t.accent,
                                );
                                ui.add_space(12.0);
                            }
                            let content_width = (ui.available_width() - 104.0).max(64.0);
                            ui.allocate_ui_with_layout(vec2(content_width, 52.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                                ui.add(egui::Label::new(RichText::new(&item.title).font(semibold(18.0)).color(t.text)).wrap());
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "{} · Excluído em {}",
                                            if item.kind == "project" { "Projeto" } else { "Arquivo" },
                                            date(item.deleted_at)
                                        ))
                                        .font(regular(13.0))
                                        .color(t.text_secondary),
                                    )
                                    .wrap(),
                                );
                            });
                            let restore = icon_button(ui, Icon::Undo, "Restaurar item", false, t);
                            if restore.clicked() {
                                state.result(store.restore(std::slice::from_ref(&item.id)), "Item restaurado ao local de origem.");
                            }
                            let more = icon_button(ui, Icon::Dots, "Opções do item excluído", false, t);
                            crate::right::menu_popup(&more).show(|ui| {
                                if ui.button("Excluir permanentemente").clicked() {
                                    state.open(Dialog::Delete { ids: vec![item.id.clone()], titles: vec![item.title.clone()] }, more.id);
                                    ui.close();
                                }
                            });
                        });
                    });
                });
                if row_index + 1 < row_count {
                    ui.separator();
                }
            }
        });
    }
    dialogs(app, ui, store, state, t)
}

fn trash_actions(ui: &mut Ui, store: &mut workspace::Store, state: &mut State, t: &Tokens) {
    ui.horizontal_wrapped(|ui| {
        let empty = ui.add_enabled(!store.data.trash.is_empty(), styled_button("Esvaziar lixeira", false, t));
        if empty.clicked() {
            state.open(
                Dialog::Delete {
                    ids: store.data.trash.iter().map(|item| item.id.clone()).collect(),
                    titles: store.data.trash.iter().map(|item| item.title.clone()).collect(),
                },
                empty.id,
            );
        }
        let enabled = !state.selected.is_empty();
        if ui.add_enabled(enabled, styled_button("Restaurar selecionados", true, t)).clicked() {
            let ids: Vec<_> = state.selected.iter().cloned().collect();
            state.result(store.restore(&ids), "Itens restaurados ao local de origem.");
            state.selected.retain(|id| store.data.trash.iter().any(|item| &item.id == id));
        }
        if enabled {
            let more = icon_button(ui, Icon::Dots, "Opções dos itens selecionados", false, t);
            crate::right::menu_popup(&more).show(|ui| {
                if ui.button("Excluir selecionados").clicked() {
                    let selected: Vec<_> = store.data.trash.iter().filter(|item| state.selected.contains(&item.id)).collect();
                    let dialog = Dialog::Delete {
                        ids: selected.iter().map(|item| item.id.clone()).collect(),
                        titles: selected.iter().map(|item| item.title.clone()).collect(),
                    };
                    state.open(dialog, more.id);
                    ui.close();
                }
            });
        }
    });
}

impl State {
    fn open(&mut self, dialog: Dialog, opener: Id) {
        self.dialog = Some(dialog);
        self.opener = Some(opener);
        self.focus_first = true;
        self.preview = None;
        self.preview_reader = None;
        self.feedback = None;
    }

    fn result(&mut self, result: Result<(), String>, success: &str) -> bool {
        match result {
            Ok(()) => {
                self.feedback = Some((success.into(), false));
                true
            }
            Err(error) => {
                self.feedback = Some((error, true));
                false
            }
        }
    }

    fn start_preview(&mut self, ctx: &egui::Context, source: PreviewSource) {
        let (send, receive) = mpsc::channel();
        let ctx = ctx.clone();
        self.preview_reader = Some(receive);
        let spawned = std::thread::Builder::new().name("designflow-library-preview".into()).spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read_preview(source)))
                .unwrap_or_else(|_| Err("Não foi possível gerar uma prévia deste arquivo.".into()));
            let _ = send.send(result);
            ctx.request_repaint();
        });
        if let Err(error) = spawned {
            self.preview_reader = None;
            self.preview = Some(Err(format!("Não foi possível iniciar a prévia: {error}")));
        }
    }

    fn receive_card_preview(&mut self, ctx: &egui::Context) {
        let result = self.card_reader.as_ref().and_then(|(key, receiver)| match receiver.try_recv() {
            Ok(result) => Some((key.clone(), result)),
            Err(mpsc::TryRecvError::Disconnected) => Some((key.clone(), Err("Prévia indisponível".into()))),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some((key, result)) = result {
            // Bound retained GPU memory when a large local catalogue is browsed.
            if self.card_previews.len() >= 64 {
                self.card_previews.clear();
            }
            let preview = result.map(|image| ctx.load_texture(format!("collection-card-{key}"), image, egui::TextureOptions::LINEAR));
            self.card_previews.insert(key, preview);
            self.card_reader = None;
        }
    }
}

fn dialogs(app: &mut FigApp, ui: &mut Ui, store: &mut workspace::Store, state: &mut State, t: &Tokens) -> Option<Action> {
    let dialog = state.dialog.clone()?;
    if let Some(reader) = &state.preview_reader {
        match reader.try_recv() {
            Ok(result) => {
                state.preview = Some(result.map(|image| ui.ctx().load_texture("designflow-library-preview", image, egui::TextureOptions::LINEAR)));
                state.preview_reader = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                state.preview = Some(Err("A leitura da prévia foi interrompida. Feche e tente novamente.".into()));
                state.preview_reader = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    let ctx = ui.ctx().clone();
    let first = std::mem::take(&mut state.focus_first);
    let mut close = false;
    let mut action = None;
    let response = egui::Modal::new(Id::unique("designflow-collections-dialog"))
        .frame(egui::Frame::new().fill(t.panel).stroke(Stroke::new(1.0, t.border)).corner_radius(16).inner_margin(24))
        .show(&ctx, |ui| {
            let width = (ctx.content_rect().width() - 80.0).clamp(120.0, 640.0);
            ui.set_width(width);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            let max_height = (ctx.content_rect().height() - 110.0).max(120.0);
            let content_height: f32 = match &dialog {
                Dialog::Resource(_) => 580.0,
                Dialog::Library(_) => 450.0,
                _ => 250.0,
            };
            egui::ScrollArea::vertical()
                .id_salt(("collections-dialog-body", std::mem::discriminant(&dialog)))
                .max_height(max_height)
                .min_scrolled_height(content_height.min(max_height))
                .show(ui, |ui| {
                    match dialog {
                        Dialog::Resource(id) => {
                            let Some(resource) = RESOURCES.iter().find(|resource| resource.template == id) else {
                                close = true;
                                return;
                            };
                            ui.label(RichText::new(resource.title).font(semibold(20.0)).color(t.text));
                            ui.add_space(8.0);
                            ui.label(RichText::new(resource.summary).font(regular(15.0)).color(t.text_secondary));
                            ui.add_space(16.0);
                            preview(ui, state, t);
                            ui.add_space(16.0);
                            ui.label(
                                RichText::new("Fonte: kit nativo do DesignFlow · Licença: MIT ou Apache-2.0")
                                    .font(regular(13.0))
                                    .color(t.text_secondary),
                            );
                            ui.label(
                                RichText::new("Os textos e imagens de exemplo pertencem ao modelo editável; não são dados da sua conta.")
                                    .font(regular(13.0))
                                    .color(t.text_secondary),
                            );
                            ui.add_space(20.0);
                            ui.horizontal_wrapped(|ui| {
                                let cancel = button(ui, "Fechar", false, t);
                                if first {
                                    cancel.request_focus();
                                }
                                close |= cancel.clicked();
                                if button(ui, "Adicionar à biblioteca", true, t).clicked() {
                                    match store.add_template_to_library(id) {
                                        Ok(_) => {
                                            state.feedback = Some(("Recurso salvo na sua biblioteca local.".into(), false));
                                            close = true;
                                        }
                                        Err(error) => state.feedback = Some((error, true)),
                                    }
                                }
                            });
                        }
                        Dialog::Library(ref id) => {
                            let Some(item) = store.data.library.iter().find(|item| &item.id == id) else {
                                close = true;
                                return;
                            };
                            ui.label(RichText::new(&item.title).font(semibold(20.0)).color(t.text));
                            ui.add_space(8.0);
                            ui.label(RichText::new(library_kind(&item.kind)).font(regular(13.0)).color(t.text_secondary));
                            ui.add_space(16.0);
                            preview(ui, state, t);
                            ui.add_space(20.0);
                            ui.horizontal_wrapped(|ui| {
                                let cancel = button(ui, "Fechar", false, t);
                                if first {
                                    cancel.request_focus();
                                }
                                close |= cancel.clicked();
                                if button(ui, "Criar projeto deste arquivo", true, t).clicked() {
                                    action = Some(Action::CreateFromLibrary(id.clone()));
                                    close = true;
                                }
                            });
                        }
                        Dialog::Rename { ref id, ref name } => {
                            ui.label(RichText::new("Renomear arquivo").font(semibold(20.0)).color(t.text));
                            ui.add_space(16.0);
                            ui.label(RichText::new("Nome").font(regular(15.0)).color(t.text));
                            let mut value = name.clone();
                            let input = ui.add_sized([width, 46.0], egui::TextEdit::singleline(&mut value).font(regular(15.0)).char_limit(120));
                            if first {
                                input.request_focus();
                            }
                            state.dialog = Some(Dialog::Rename { id: id.clone(), name: value.clone() });
                            let valid = !value.trim().is_empty() && !value.chars().any(char::is_control);
                            if !valid {
                                ui.colored_label(t.danger, "Informe um nome de 1 a 120 caracteres.");
                            }
                            ui.add_space(20.0);
                            ui.horizontal_wrapped(|ui| {
                                close |= button(ui, "Cancelar", false, t).clicked();
                                let save = ui.add_enabled(valid, styled_button("Salvar nome", true, t));
                                if (save.clicked() || (input.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))) && valid {
                                    close = state.result(store.rename_library(id, &value), "Nome atualizado.");
                                }
                            });
                        }
                        Dialog::TrashLibrary { ref id, ref title } => {
                            ui.label(RichText::new("Mover para a lixeira?").font(semibold(20.0)).color(t.text));
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(format!("\"{title}\" sairá da biblioteca. Você poderá restaurá-lo pela Lixeira."))
                                    .font(regular(15.0))
                                    .color(t.text_secondary),
                            );
                            ui.add_space(20.0);
                            ui.horizontal_wrapped(|ui| {
                                let cancel = button(ui, "Cancelar", false, t);
                                if first {
                                    cancel.request_focus();
                                }
                                close |= cancel.clicked();
                                if button(ui, "Mover para lixeira", true, t).clicked() {
                                    close = state.result(store.trash_library(id), "Arquivo movido para a lixeira.");
                                }
                            });
                        }
                        Dialog::Delete { ref ids, ref titles } => {
                            ui.label(RichText::new("Excluir permanentemente?").font(semibold(20.0)).color(t.text));
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(format!(
                                    "A exclusão removerá {} {} deste espaço e apagará os arquivos do computador. Esta ação não pode ser desfeita.",
                                    ids.len(),
                                    if ids.len() == 1 { "item" } else { "itens" }
                                ))
                                .font(regular(15.0))
                                .color(t.text_secondary),
                            );
                            ui.add_space(12.0);
                            for title in titles.iter().take(8) {
                                ui.label(RichText::new(title).font(regular(13.0)).color(t.text));
                            }
                            if titles.len() > 8 {
                                ui.label(
                                    RichText::new(format!("E mais {} itens selecionados.", titles.len() - 8))
                                        .font(regular(13.0))
                                        .color(t.text_secondary),
                                );
                            }
                            ui.add_space(20.0);
                            ui.horizontal_wrapped(|ui| {
                                let cancel = button(ui, "Cancelar", false, t);
                                if first {
                                    cancel.request_focus();
                                }
                                close |= cancel.clicked();
                                let delete = ui.add(
                                    egui::Button::new(RichText::new("Excluir permanentemente").font(semibold(15.0)).color(t.panel))
                                        .min_size(vec2(0.0, 44.0))
                                        .corner_radius(12)
                                        .fill(t.danger)
                                        .stroke(Stroke::NONE),
                                );
                                if delete.clicked() {
                                    let result = close_projects_before_delete(app, store, ids).and_then(|()| store.delete_permanently(ids));
                                    let succeeded = state.result(result, "Itens excluídos permanentemente.");
                                    state.selected.retain(|id| store.data.trash.iter().any(|item| &item.id == id));
                                    // A partial failure must keep its failed IDs only, so a retry never
                                    // asks to delete or counts items which are already gone.
                                    let remaining: Vec<_> = store.data.trash.iter().filter(|item| ids.contains(&item.id)).collect();
                                    close = succeeded || remaining.is_empty();
                                    if !close {
                                        state.dialog = Some(Dialog::Delete {
                                            ids: remaining.iter().map(|item| item.id.clone()).collect(),
                                            titles: remaining.iter().map(|item| item.title.clone()).collect(),
                                        });
                                    }
                                }
                            });
                        }
                    }
                    if let Some((message, true)) = &state.feedback {
                        ui.add_space(16.0);
                        ui.label(RichText::new(message).font(regular(13.0)).color(t.danger));
                        if button(ui, "Recarregar dados locais", false, t).clicked() {
                            reload_store(store, state);
                        }
                    }
                });
        });
    if close || response.should_close() {
        state.dialog = None;
        state.preview_reader = None;
        state.preview = None;
        if let Some(opener) = state.opener.take() {
            ctx.memory_mut(|memory| memory.request_focus(opener));
        }
    }
    action
}

enum PreviewSource {
    Template(&'static str),
    File(PathBuf),
}

fn read_preview(source: PreviewSource) -> Result<egui::ColorImage, String> {
    let document = match source {
        PreviewSource::Template(id) => designflow_engine::templates::find(id)
            .ok_or("Recurso não disponível.")?
            .build(designflow_color::Color::WHITE)
            .map_err(|error| error.to_string())?,
        PreviewSource::File(path) => {
            let metadata = std::fs::metadata(&path).map_err(|_| "Não foi possível ler o arquivo local.")?;
            if !metadata.is_file() || metadata.len() > workspace::MAX_IMPORT_BYTES {
                return Err("A prévia requer um arquivo regular de até 256 MiB.".into());
            }
            if let Some(picture) = designflow_format::thumbnail::file_thumbnail(&path, 960) {
                return Ok(egui::ColorImage::from_rgba_unmultiplied([picture.width as usize, picture.height as usize], &picture.rgba));
            }
            let file = std::fs::File::open(&path).map_err(|_| "Não foi possível ler o arquivo local.")?;
            let mut bytes = Vec::new();
            file.take(workspace::MAX_IMPORT_BYTES + 1).read_to_end(&mut bytes).map_err(|_| "Não foi possível ler a prévia.")?;
            if bytes.len() as u64 > workspace::MAX_IMPORT_BYTES {
                return Err("Este arquivo excede o limite de leitura da prévia (256 MiB).".into());
            }
            let hint = path.file_name().and_then(|value| value.to_str()).unwrap_or("arquivo.designflow");
            designflow_engine::cmd::file::load_bytes(&bytes, hint).map_err(|error| format!("Não foi possível gerar a prévia: {error}"))?.0
        }
    };
    let page = document.first_page().ok_or("O documento não contém páginas.")?;
    let frames = document.node(page).ok_or("Página não encontrada.")?.children.clone();
    let bounds = frames
        .iter()
        .filter_map(|id| designflow_render::export_bounds(&document, *id))
        .reduce(|a, b| a.union(b))
        .ok_or("Este arquivo ainda não tem conteúdo visual para mostrar.")?;
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return Err("O arquivo não tem uma área visual válida.".into());
    }
    let scale = (960.0 / bounds.width()).min(640.0 / bounds.height()).min(1.0);
    if !scale.is_finite() || scale <= 0.0 {
        return Err("Largura ou altura inválida para a prévia.".into());
    }
    let image = designflow_render::render_area(&document, &frames, bounds, scale).ok_or("Não foi possível renderizar esta prévia.")?;
    Ok(crate::render_worker::screen_image(&image, document.color_profile))
}

fn preview(ui: &mut Ui, state: &State, t: &Tokens) {
    egui::Frame::new().fill(t.canvas).stroke(Stroke::new(1.0, t.border)).corner_radius(12).inner_margin(16).show(ui, |ui| {
        let width = (ui.available_width() - 1.0).max(1.0);
        ui.set_min_width(width);
        match &state.preview {
            Some(Ok(texture)) => {
                ui.add(
                    egui::Image::new(texture)
                        .fit_to_exact_size(texture.size_vec2() * (width / texture.size_vec2().x).min(320.0 / texture.size_vec2().y)),
                );
            }
            Some(Err(error)) => {
                ui.label(RichText::new(error).font(regular(13.0)).color(t.text_secondary));
            }
            None => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("Gerando prévia do arquivo…").font(regular(13.0)).color(t.text_secondary));
                });
            }
        }
    });
}

fn heading(ui: &mut Ui, title: &str, description: &str, t: &Tokens) {
    ui.label(RichText::new(title).font(semibold(34.0)).color(t.text));
    ui.add_space(8.0);
    ui.add(egui::Label::new(RichText::new(description).font(regular(15.0)).color(t.text_secondary)).wrap());
    ui.add_space(32.0);
}

fn filters(ui: &mut Ui, selected: &mut usize, names: &[&str], t: &Tokens) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
        for (index, name) in names.iter().enumerate() {
            let active = *selected == index;
            let font = regular(15.0);
            let text = ui.painter().layout_no_wrap((*name).to_owned(), font.clone(), t.text);
            let (rect, response) = ui.allocate_exact_size(vec2(text.size().x + 66.0, 46.0), egui::Sense::click());
            ui.painter().rect_filled(
                rect,
                12,
                if active {
                    t.selected_row
                } else if response.hovered() {
                    t.hover
                } else {
                    t.panel
                },
            );
            ui.painter().rect_stroke(
                rect,
                12,
                Stroke::new(
                    if response.has_focus() { 2.0 } else { 1.0 },
                    if response.has_focus() {
                        t.focus
                    } else if active {
                        t.selected_row
                    } else {
                        t.border
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let color = if active { t.accent_text } else { t.text };
            icons::paint(
                ui.painter(),
                egui::Rect::from_center_size(egui::pos2(rect.left() + 24.0, rect.center().y), vec2(20.0, 20.0)),
                filter_icon(name),
                color,
            );
            ui.painter().text(egui::pos2(rect.left() + 44.0, rect.center().y), egui::Align2::LEFT_CENTER, name, font, color);
            crate::a11y::toggle(&response, name, active);
            if response.clicked() {
                *selected = index;
            }
        }
    });
}

fn styled_button<'a>(title: &'a str, primary: bool, t: &Tokens) -> egui::Button<'a> {
    egui::Button::new(RichText::new(title).font(semibold(15.0)).color(if primary { t.accent_on } else { t.text }))
        .wrap_mode(egui::TextWrapMode::Extend)
        .min_size(vec2(0.0, 44.0))
        .corner_radius(12)
        .fill(if primary { t.accent } else { t.panel })
        .stroke(if primary { Stroke::NONE } else { Stroke::new(1.0, t.border) })
}

fn filter_icon(name: &str) -> Icon {
    match name {
        "Favoritos" => Icon::Star,
        "Uploads" => Icon::Import,
        "Logos" | "Componentes" => Icon::Layers,
        "Cores" => Icon::Fill,
        "Imagens" => Icon::Image,
        "Ilustrações" => Icon::Pen,
        "Mockups" => Icon::Panels,
        "Projetos" => Icon::Page,
        "Arquivos" => Icon::Page,
        "Coleções" => Icon::Library,
        _ => Icon::Grid,
    }
}

fn resource_card(ui: &mut Ui, state: &mut State, resource: &Resource, t: &Tokens) {
    let wide = ui.available_width() >= 480.0;
    let copy_width = if wide { (ui.available_width() * 0.48).max(200.0) } else { ui.available_width() };
    let mut open = None;
    let mut copy = |ui: &mut Ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let (badge, _) = ui.allocate_exact_size(vec2(40.0, 40.0), egui::Sense::hover());
        ui.painter().rect_filled(badge, 10, t.selected_row);
        icons::paint(ui.painter(), badge.shrink(10.0), resource.icon, t.accent);
        ui.add_space(10.0);
        ui.add(egui::Label::new(RichText::new(resource.title).font(semibold(23.0)).color(t.text)).wrap());
        ui.add_space(6.0);
        ui.add(egui::Label::new(RichText::new(resource.summary).font(regular(15.0)).color(t.text_secondary)).wrap());
        ui.add_space(14.0);
        let response = button(ui, "Visualizar recurso", false, t);
        if response.clicked() {
            open = Some(response.id);
        }
    };
    if wide {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 20.0;
            ui.allocate_ui_with_layout(vec2(copy_width, 208.0), egui::Layout::top_down(egui::Align::Min), |ui| copy(ui));
            let size = vec2(ui.available_width().max(1.0), 208.0);
            picture(ui, state, resource.template.to_owned(), Some(PreviewSource::Template(resource.template)), resource.icon, size, t);
        });
    } else {
        card_picture(ui, state, resource.template.to_owned(), Some(PreviewSource::Template(resource.template)), resource.icon, t);
        ui.add_space(18.0);
        copy(ui);
    }
    if let Some(opener) = open {
        state.open(Dialog::Resource(resource.template), opener);
        state.start_preview(ui.ctx(), PreviewSource::Template(resource.template));
    }
}

fn button(ui: &mut Ui, title: &str, primary: bool, t: &Tokens) -> Response {
    ui.add(styled_button(title, primary, t))
}

fn import_button(ui: &mut Ui, t: &Tokens) -> Response {
    let response = ui.add(styled_button("\u{2003}\u{2003}Adicionar à biblioteca", true, t));
    icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(egui::pos2(response.rect.left() + 22.0, response.rect.center().y), vec2(20.0, 20.0)),
        Icon::Plus,
        t.accent_on,
    );
    crate::a11y::button(&response, "Adicionar à biblioteca");
    response
}

fn icon_button(ui: &mut Ui, icon: Icon, label: &str, active: bool, t: &Tokens) -> Response {
    let response = ui.add(
        egui::Button::new("")
            .min_size(vec2(44.0, 44.0))
            .corner_radius(12)
            .fill(if active { t.selected_row } else { t.panel })
            .stroke(Stroke::new(1.0, t.border)),
    );
    icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(response.rect.center(), vec2(20.0, 20.0)),
        icon,
        if active { t.accent } else { t.icon_secondary },
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    response.on_hover_text(label)
}

fn card(ui: &mut Ui, width: f32, t: &Tokens, content: impl FnOnce(&mut Ui)) {
    egui::Frame::new().fill(t.panel).stroke(Stroke::new(1.0, t.border)).corner_radius(16).inner_margin(22).show(ui, |ui| {
        // Frame inherits its parent's row layout. Give every card an explicit
        // vertical body and a width including padding plus its two border pixels.
        let inner = (width - 46.0).max(1.0);
        ui.set_width(inner);
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(inner);
            content(ui);
        });
    });
}

fn library_card(ui: &mut Ui, width: f32, t: &Tokens, content: impl FnOnce(&mut Ui)) {
    egui::Frame::new().fill(t.panel).stroke(Stroke::new(1.0, t.border)).corner_radius(16).inner_margin(8).show(ui, |ui| {
        let inner = (width - 18.0).max(1.0);
        ui.set_width(inner);
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(inner);
            content(ui);
        });
    });
}

fn icon_tile(ui: &mut Ui, icon: Icon, t: &Tokens) {
    let (rect, _) = ui.allocate_exact_size(vec2(48.0, 48.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 12, t.selected_row);
    icons::paint(ui.painter(), rect.shrink(13.0), icon, t.accent);
}

fn card_picture(ui: &mut Ui, state: &mut State, key: String, source: Option<PreviewSource>, icon: Icon, t: &Tokens) -> egui::Rect {
    picture(ui, state, key, source, icon, vec2(ui.available_width().max(1.0), (ui.available_width() * 0.48).clamp(132.0, 208.0)), t)
}

/// Shared, bounded asynchronous previews for the real project catalogue.
pub(super) fn document_preview(ui: &mut Ui, state: &mut State, path: &Path, modified: u64, size: egui::Vec2, t: &Tokens) {
    state.receive_card_preview(ui.ctx());
    picture(ui, state, format!("project:{}:{modified}", path.display()), Some(PreviewSource::File(path.to_owned())), Icon::Page, size, t);
}

fn picture(ui: &mut Ui, state: &mut State, key: String, source: Option<PreviewSource>, icon: Icon, size: egui::Vec2, t: &Tokens) -> egui::Rect {
    let width = size.x.max(1.0);
    let height = size.y.max(1.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return rect;
    }
    if !state.card_previews.contains_key(&key) && state.card_reader.is_none() {
        if let Some(source) = source {
            let (send, receive) = mpsc::channel();
            let ctx = ui.ctx().clone();
            let spawned = std::thread::Builder::new().name("designflow-card-preview".into()).spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read_preview(source)))
                    .unwrap_or_else(|_| Err("Prévia indisponível".into()));
                let _ = send.send(result);
                ctx.request_repaint();
            });
            if spawned.is_ok() {
                state.card_reader = Some((key.clone(), receive));
            } else {
                state.card_previews.insert(key.clone(), Err("Prévia indisponível".into()));
            }
        } else {
            state.card_previews.insert(key.clone(), Err("Arquivo indisponível".into()));
        }
    }
    ui.painter().rect_filled(rect, 12, t.canvas);
    if let Some(Ok(texture)) = state.card_previews.get(&key) {
        let scale = ((width - 16.0).max(1.0) / texture.size_vec2().x).min((height - 16.0).max(1.0) / texture.size_vec2().y);
        let image = egui::Rect::from_center_size(rect.center(), texture.size_vec2() * scale);
        ui.painter().image(texture.id(), image, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    } else {
        icons::paint(ui.painter(), egui::Rect::from_center_size(rect.center(), vec2(32.0, 32.0)), icon, t.accent);
        let text = if state.card_previews.contains_key(&key) { "Prévia indisponível" } else { "Preparando prévia…" };
        ui.painter().text(rect.center_bottom() - vec2(0.0, 16.0), egui::Align2::CENTER_BOTTOM, text, regular(13.0), t.text_secondary);
    }
    rect
}

fn empty(ui: &mut Ui, title: &str, description: &str, icon: Icon, t: &Tokens) {
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        icon_tile(ui, icon, t);
        ui.add_space(20.0);
        ui.label(RichText::new(title).font(semibold(20.0)).color(t.text));
        ui.add_space(8.0);
        ui.add(egui::Label::new(RichText::new(description).font(regular(15.0)).color(t.text_secondary)).wrap());
    });
}

fn feedback(ui: &mut Ui, store: &mut workspace::Store, state: &mut State, t: &Tokens) {
    if let Some((message, error)) = state.feedback.clone() {
        egui::Frame::new().fill(t.panel).stroke(Stroke::new(1.0, if error { t.danger } else { t.border })).corner_radius(12).inner_margin(12).show(
            ui,
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(if error { format!("Erro: {message}") } else { message }).font(regular(13.0)).color(if error {
                        t.danger
                    } else {
                        t.success
                    }));
                    if button(ui, "Fechar aviso", false, t).clicked() {
                        state.feedback = None;
                    }
                    if error && button(ui, "Recarregar dados locais", false, t).clicked() {
                        reload_store(store, state);
                    }
                });
            },
        );
        ui.add_space(16.0);
    }
}

fn reload_store(store: &mut workspace::Store, state: &mut State) {
    if state.result(store.reload(), "Dados locais recarregados. Você pode tentar a ação novamente.") {
        state.selected.retain(|id| store.data.trash.iter().any(|item| &item.id == id));
        state.card_previews.clear();
        state.card_reader = None;
    }
}

/// A trashed document can still have been opened from the native recent-files menu.
/// Recheck the live session immediately before deleting; the earlier soft-delete guard
/// cannot guarantee that the file has stayed closed. Only Store removes managed files.
fn close_projects_before_delete(app: &mut FigApp, store: &workspace::Store, ids: &[String]) -> Result<(), String> {
    let paths: Vec<_> = store
        .data
        .trash
        .iter()
        .filter(|item| ids.contains(&item.id))
        .filter_map(|item| {
            if let workspace::TrashedRecord::Project(project) = &item.original {
                let path = store.root.join(&project.path);
                Some((std::fs::canonicalize(&path).unwrap_or(path), project.title.clone()))
            } else {
                None
            }
        })
        .collect();
    let open: Vec<_> = app
        .session
        .docs
        .iter()
        .enumerate()
        .filter_map(|(index, document)| {
            let path = PathBuf::from(document.path.as_ref()?);
            let path = std::fs::canonicalize(&path).unwrap_or(path);
            paths.iter().find(|(managed, _)| managed == &path).map(|(_, title)| (index, document.uid, document.is_dirty(), title.clone()))
        })
        .collect();
    let dirty: Vec<_> = open.iter().filter(|(_, _, dirty, _)| *dirty).map(|(_, _, _, title)| title.as_str()).collect();
    if !dirty.is_empty() {
        return Err(format!(
            "Salve as alterações e feche os projetos antes de excluir permanentemente: {}. Nenhum item foi excluído.",
            dirty.join(", ")
        ));
    }
    for (_, uid, _, title) in open.into_iter().rev() {
        // Closing a main document also closes its branch tabs. Resolve each UID again
        // rather than using an index shifted by those removals.
        if let Some(index) = app.session.docs.iter().position(|document| document.uid == uid) {
            if app.run("file.close", serde_json::json!({"index": index})).is_none() {
                return Err(format!("Não foi possível fechar \"{title}\". Nenhum arquivo foi excluído; tente novamente."));
            }
        }
    }
    Ok(())
}

fn columns(width: f32, max: usize) -> usize {
    (((width + 22.0) / 330.0).floor() as usize).clamp(1, max)
}
fn card_width(width: f32, columns: usize) -> f32 {
    ((width - 22.0 * (columns - 1) as f32) / columns as f32).max(1.0)
}
fn library_kind(kind: &str) -> &str {
    match kind {
        "collection" => "Coleção editável · Arquivo local",
        "image" => "Imagem importada · Arquivo local",
        "logo" => "Logo · Arquivo local",
        "palette" => "Paleta · Arquivo local",
        _ => "Arquivo importado · Arquivo local",
    }
}
fn date(timestamp: u64) -> String {
    i64::try_from(timestamp)
        .ok()
        .and_then(|value| chrono::DateTime::from_timestamp(value, 0))
        .map(|value| value.with_timezone(&chrono::Local).format("%d/%m/%Y às %H:%M").to_string())
        .unwrap_or_else(|| "data indisponível".into())
}
