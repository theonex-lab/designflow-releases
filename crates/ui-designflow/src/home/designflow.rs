//! The native DesignFlow hub entry point; data, page state and views are separate modules.
pub use super::hub::State;
pub(super) fn show(app: &mut crate::FigApp, ui: &mut egui::Ui) {
    super::hub::show(app, ui);
}
