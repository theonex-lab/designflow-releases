//! The editor's dedicated shell. Every action goes through the existing command catalog.
use crate::{
    FigApp,
    icons::{self, Icon},
    theme,
};
use egui::{Align2, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::json;

pub(crate) fn show(app: &mut FigApp, root: &mut Ui) {
    let t = theme::Tokens::get(root.ctx());
    let compact = root.available_width() < theme::system::COMPACT;
    egui::Panel::top("designflow_editor_header")
        .exact_size(if compact { 64.0 } else { 60.0 })
        .frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(12, 6)))
        .show(root, |ui| {
            let r = ui.max_rect();
            let cy = r.center().y;
            let mark = Rect::from_center_size(pos2(r.min.x + 22.0, cy), vec2(44.0, 44.0));
            let response = ui.interact(mark, ui.scope_id().with("designflow_editor_home"), Sense::click()).on_hover_text("Início");
            theme::paint_app_icon(ui, mark.shrink(10.0));
            crate::a11y::button(&response, "Início");
            focus_ring(ui, &response, mark, &t);
            if response.clicked() {
                crate::home::navigate(app, "/");
            }
            let menu = Rect::from_center_size(pos2(r.min.x + if compact { 74.0 } else { 123.0 }, cy), vec2(if compact { 44.0 } else { 138.0 }, 44.0));
            let menu_response = ui.interact(menu, ui.scope_id().with("designflow_editor_menu"), Sense::click());
            crate::a11y::menu_button(&menu_response, "Menu do DesignFlow", crate::a11y::menu_open(&menu_response));
            focus_ring(ui, &menu_response, menu, &t);
            if menu_response.hovered() {
                ui.painter().rect_filled(menu, 8, t.hover);
            }
            if compact {
                icons::paint(ui.painter(), Rect::from_center_size(menu.center(), vec2(20.0, 20.0)), Icon::Menu, t.icon);
            } else {
                ui.painter().text(menu.left_center() + vec2(8.0, 0.0), Align2::LEFT_CENTER, "DesignFlow", theme::designflow_semibold(15.0), t.text);
                icons::paint(ui.painter(), Rect::from_center_size(menu.right_center() - vec2(13.0, 0.0), vec2(10.0, 10.0)), Icon::Chevron, t.icon);
            }
            editor_menu(app, &menu_response);

            // Actions stay in a fixed, bounded region. On a phone, the same commands live in
            // the menu and the export button keeps a text alternative for assistive technology.
            let mut right = r.max.x;
            if !compact {
                let rect = Rect::from_center_size(pos2(right - 22.0, cy), vec2(44.0, 44.0));
                account(app, ui, &t, rect);
                right -= 52.0;
            }
            let export_w = if compact { 44.0 } else { 108.0 };
            let export_h = 44.0;
            let export = Rect::from_min_size(pos2(right - export_w, cy - export_h * 0.5), vec2(export_w, export_h));
            let exportable = can_export(app);
            let export_response =
                ui.add_enabled_ui(exportable, |ui| ui.interact(export, ui.scope_id().with("designflow_editor_export"), Sense::click())).inner;
            crate::a11y::button(&export_response, "Exportar");
            ui.painter().rect_filled(export, 8, if exportable { if export_response.hovered() { t.accent_hover } else { t.accent } } else { t.field });
            let export_ink = if exportable { t.accent_on } else { t.text_secondary };
            focus_ring(ui, &export_response, export, &t);
            if compact {
                icons::paint(ui.painter(), Rect::from_center_size(export.center(), vec2(18.0, 18.0)), Icon::Export, export_ink);
            } else {
                icons::paint(ui.painter(), Rect::from_center_size(pos2(export.left() + 22.0, cy), vec2(16.0, 16.0)), Icon::Export, export_ink);
                ui.painter().text(pos2(export.left() + 38.0, cy), Align2::LEFT_CENTER, "Exportar", theme::designflow_semibold(13.0), export_ink);
            }
            if export_response.clicked() {
                crate::menus::invoke(app, "file.export");
            }
            export_response.on_hover_text(if exportable { "Exportar seleção ou página" } else { "Adicione uma camada antes de exportar" });
            right = export.left() - 8.0;
            let zoom = Rect::from_min_size(pos2(right - 66.0, cy - export_h * 0.5), vec2(66.0, export_h));
            crate::right::zoom_control(app, ui, zoom);
            right = zoom.left() - 8.0;
            if r.width() >= 540.0 {
                let play = Rect::from_center_size(pos2(right - 22.0, cy), vec2(44.0, 44.0));
                let can_present = can_present(app);
                let play_response =
                    ui.add_enabled_ui(can_present, |ui| ui.interact(play, ui.scope_id().with("designflow_editor_present"), Sense::click())).inner;
                crate::a11y::button(&play_response, "Visualizar apresentação");
                focus_ring(ui, &play_response, play, &t);
                if play_response.hovered() {
                    ui.painter().rect_filled(play, 8, t.hover);
                }
                icons::paint(ui.painter(), play.shrink(11.0), Icon::Play, if can_present { t.icon } else { t.text_secondary });
                if play_response.clicked() {
                    app.run("proto.present", json!({}));
                }
                play_response.on_hover_text(if can_present { "Visualizar apresentação" } else { "Crie um quadro para apresentar" });
                right = play.left() - 8.0;
            }

            let mut left = menu.right() + if compact { 8.0 } else { 18.0 };
            if r.width() >= 1080.0 {
                for (label, section) in
                    [("Arquivo", "File"), ("Editar", "Edit"), ("Objeto", "Object"), ("Texto", "Text"), ("Visualizar", "View"), ("Ajuda", "Help")]
                {
                    let width = if label == "Visualizar" { 74.0 } else { 62.0 };
                    let rect = Rect::from_min_size(pos2(left, cy - 20.0), vec2(width, 40.0));
                    let response = ui.interact(rect, ui.scope_id().with(("designflow_editor_section", section)), Sense::click());
                    crate::a11y::menu_button(&response, label, app.ui.main_menu);
                    focus_ring(ui, &response, rect, &t);
                    if response.hovered() {
                        ui.painter().rect_filled(rect, 7, t.hover);
                    }
                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, theme::designflow_regular(13.0), t.text);
                    if response.clicked() || (app.ui.main_menu && response.hovered()) {
                        crate::menus::open_editor_section(app, &response, section);
                    }
                    left += width + 2.0;
                }
                if right - left > 170.0 {
                    let status = if app.session.active_read_only().is_some() {
                        "Somente leitura"
                    } else {
                        app.session.ds().map_or("Ainda não salvo", |d| {
                            if d.is_dirty() {
                                "Alterações não salvas"
                            } else if d.path.is_some() {
                                "Salvo neste computador"
                            } else {
                                "Ainda não salvo"
                            }
                        })
                    };
                    ui.painter().text(pos2(right - 8.0, cy), Align2::RIGHT_CENTER, status, theme::designflow_regular(13.0), t.text_secondary);
                }
            } else if right - left > 50.0 {
                let name = app.session.doc().map(|d| d.name.clone()).unwrap_or_default();
                let title = crate::widgets::elide_font(ui, &name, right - left, theme::designflow_regular(13.0));
                ui.painter().text(pos2(left, cy), Align2::LEFT_CENTER, title, theme::designflow_regular(13.0), t.text_secondary);
            }
            ui.painter().line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0, t.border));
        });
}

fn editor_menu(app: &mut FigApp, response: &egui::Response) {
    crate::right::menu_popup(response).width(250.0).show(|ui| {
        if crate::right::menu_item(ui, "Início", "") {
            crate::home::navigate(app, "/");
            ui.close();
        }
        for (label, command) in [
            ("Salvar", "file.save"),
            ("Salvar como…", "file.saveAs"),
            ("Abrir arquivo…", "file.open"),
            ("Exportar…", "file.export"),
            ("Visualizar apresentação", "proto.present"),
            ("Buscar ações", "app.actions"),
            ("Sobre o DesignFlow", "help.about"),
        ] {
            let enabled = crate::menus::command_enabled(app, command)
                && match command {
                    "file.export" => can_export(app),
                    "proto.present" => can_present(app),
                    _ => true,
                };
            if ui.add_enabled_ui(enabled, |ui| crate::right::menu_item(ui, label, crate::shortcuts::shortcut(command).unwrap_or(""))).inner {
                crate::menus::invoke(app, command);
                ui.close();
            }
        }
        if crate::right::menu_item(ui, "Configurações", "") {
            crate::home::navigate(app, "/configuracoes");
            ui.close();
        }
        if crate::right::menu_item(ui, "Minha conta", "") {
            crate::home::navigate(app, "/conta");
            ui.close();
        }
        if crate::right::menu_item(ui, "Menu completo", "") {
            app.ui.main_menu = true;
            ui.close();
        }
        if let Some(email) = app.auth.as_ref().and_then(|auth| auth.account_email()) {
            ui.separator();
            ui.label(email);
        }
        if crate::right::menu_item(ui, "Sair da conta", "") {
            app.run("auth.signOut", json!({}));
            ui.close();
        }
    });
}

fn account(app: &mut FigApp, ui: &mut Ui, t: &theme::Tokens, rect: Rect) {
    let email = app.auth.as_ref().and_then(|auth| auth.account_email()).unwrap_or_default().to_owned();
    let response = ui.interact(rect, ui.scope_id().with("designflow_editor_account"), Sense::click());
    crate::a11y::menu_button(&response, "Minha conta", crate::a11y::menu_open(&response));
    ui.painter().circle_filled(rect.center(), 16.0, if response.hovered() { t.selected_row } else { t.field });
    icons::paint(ui.painter(), Rect::from_center_size(rect.center(), vec2(20.0, 20.0)), Icon::User, t.icon);
    focus_ring(ui, &response, rect, t);
    crate::right::menu_popup(&response).width(240.0).show(|ui| {
        ui.label(&email);
        ui.separator();
        if crate::right::menu_item(ui, "Minha conta", "") {
            crate::home::navigate(app, "/conta");
            ui.close();
        }
        if crate::right::menu_item(ui, "Configurações", "") {
            crate::home::navigate(app, "/configuracoes");
            ui.close();
        }
        if crate::right::menu_item(ui, "Início", "") {
            crate::home::navigate(app, "/");
            ui.close();
        }
        if crate::right::menu_item(ui, "Sair da conta", "") {
            app.run("auth.signOut", json!({}));
            crate::windows::repaint_others(app, ui.ctx());
            ui.close();
        }
    });
}

fn focus_ring(ui: &Ui, response: &egui::Response, rect: Rect, t: &theme::Tokens) {
    if response.has_focus() {
        theme::paint_focus_ring(ui, rect.shrink(1.0), 10, t);
    }
}

fn can_export(app: &FigApp) -> bool {
    app.session.ds().is_ok_and(|d| !d.selection.is_empty() || d.doc.node(d.page).is_some_and(|page| !page.children.is_empty()))
}

fn can_present(app: &FigApp) -> bool {
    app.session.doc().is_ok_and(|doc| !designflow_engine::board::is_board(doc) && doc.nodes().any(|node| node.kind.is_frame_like()))
}
