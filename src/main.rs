//! SmartExplorer — Explorateur de fichiers multiplateforme (Windows, Linux, macOS)
//! écrit en Rust avec une interface inspirée de l'Explorateur Windows.

// On évite l'ouverture d'une console noire sur Windows en mode release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod credentials;
mod fs;
mod icons;
mod remote;
mod ui;

use eframe::egui;

/// Implémentation `eframe::App` : délègue le rendu au module `ui`.
struct ExplorerAppWrapper {
    app: app::ExplorerApp,
}

impl eframe::App for ExplorerAppWrapper {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ui::show(&mut self.app, &ctx, ui);
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SmartExplorer")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([640.0, 420.0]),
        ..Default::default()
    };

    eframe::run_native(
        "SmartExplorer",
        options,
        Box::new(|_cc| {
            Ok(Box::new(ExplorerAppWrapper {
                app: app::ExplorerApp::new(),
            }))
        }),
    )
}
