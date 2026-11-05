//! Small native counterparts of the supplied visual component contracts.
use crate::{
    a11y,
    icons::{self, Icon},
    theme::{self, Tokens},
};
use egui::{Color32, Margin, Rect, RichText, Sense, Stroke, Ui, vec2};

pub(super) fn title(ui: &mut Ui, label: &str, description: &str, t: &Tokens) {
    let size = if ui.available_width() < 500.0 { 26.0 } else { 34.0 };
    ui.label(RichText::new(label).font(theme::designflow_semibold(size)).color(t.text));
    if !description.is_empty() {
        ui.add_space(8.0);
        muted(ui, description, t);
    }
    ui.add_space(32.0);
}
pub(super) fn muted(ui: &mut Ui, text: &str, t: &Tokens) {
    ui.label(RichText::new(text).font(theme::designflow_regular(15.0)).color(t.text_secondary));
}
pub(super) fn button(ui: &mut Ui, label: &str, primary: bool, enabled: bool, t: &Tokens) -> egui::Response {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).font(theme::designflow_semibold(15.0)).color(if primary { Color32::WHITE } else { t.text }))
            .wrap_mode(egui::TextWrapMode::Extend)
            .fill(if primary { t.accent } else { t.panel })
            .stroke(Stroke::new(1.0, if primary { t.accent } else { t.border }))
            .corner_radius(12)
            .min_size(vec2(0.0, 44.0)),
    );
    a11y::button(&response, label);
    response
}
pub(super) fn chip(ui: &mut Ui, label: &str, active: bool, t: &Tokens) -> bool {
    let r = ui.add(
        egui::Button::new(RichText::new(label).font(theme::designflow_regular(14.0)).color(if active { t.accent_text } else { t.text }))
            .wrap_mode(egui::TextWrapMode::Extend)
            .fill(if active { t.selected_row } else { t.panel })
            .stroke(Stroke::new(1.0, if active { t.selected_row } else { t.border }))
            .corner_radius(12)
            .min_size(vec2(0.0, 44.0)),
    );
    a11y::toggle(&r, label, active);
    r.clicked()
}
pub(super) fn icon_button(ui: &mut Ui, icon: Icon, label: &str, active: bool, t: &Tokens) -> egui::Response {
    let (r, response) = ui.allocate_exact_size(vec2(40.0, 44.0), Sense::click());
    a11y::button(&response, label);
    if response.hovered() || active {
        ui.painter().rect_filled(r, 10, t.selected_row);
    }
    icons::paint(ui.painter(), Rect::from_center_size(r.center(), vec2(20.0, 20.0)), icon, if active { t.accent } else { t.icon });
    if response.has_focus() {
        theme::paint_focus_ring(ui, r, 10, t);
    }
    response.on_hover_text(label)
}
pub(super) fn card<R>(ui: &mut Ui, t: &Tokens, content: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new().fill(t.panel).stroke(Stroke::new(1.0, t.border)).corner_radius(16).inner_margin(Margin::same(22)).show(ui, content).inner
}
pub(super) fn status(ui: &mut Ui, message: &str, error: bool, t: &Tokens) {
    let response = ui.label(RichText::new(message).font(theme::designflow_regular(14.0)).color(if error { t.danger } else { t.text_secondary }));
    a11y::status(&response, if error { "Erro" } else { "Estado" }, message);
}
