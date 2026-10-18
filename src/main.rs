//! SmartExplorer — Explorateur de fichiers multiplateforme (Windows, Linux, macOS)
//! écrit en Rust avec une interface inspirée de l'Explorateur Windows.

// On évite l'ouverture d'une console noire sur Windows en mode release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod animation;
mod credentials;
mod fs;
mod icons;
mod remote;
mod ui;

use eframe::egui;

/// Force l'icône du Dock macOS à partir de l'`.icns` embarqué dans le bundle.
///
/// winit/eframe ne définit pas l'icône d'application native : sans cette
/// étape, macOS affiche l'icône par défaut (binaire) dans le Dock. On charge
/// `Contents/Resources/AppIcon.icns` et on l'assigne à
/// `NSApplication.applicationIconImage`. Sans bundle (`cargo run`), on no-op.
///
/// Appelée deux fois au besoin : au début de `main()` **et** au premier frame
/// — LaunchServices écrase l'icône réglée avant la fin du lancement, il faut
/// donc la ré-appliquer une fois l'app lancée.
#[cfg(target_os = "macos")]
fn set_dock_icon() {
    use objc2::AnyThread;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSString;

    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(_) => return,
    };
    let icns = match exe.parent().and_then(|macos| macos.parent()) {
        Some(contents) => contents.join("Resources").join("AppIcon.icns"),
        None => return,
    };
    let path = icns.to_string_lossy().to_string();
    unsafe {
        let Some(mtm) = objc2::MainThreadMarker::new() else {
            return;
        };
        let ns_path = NSString::from_str(&path);
        let image = NSImage::initWithContentsOfFile(NSImage::alloc(), &ns_path);
        match image {
            Some(image) => {
                let app = NSApplication::sharedApplication(mtm);
                app.setApplicationIconImage(Some(&image));
                #[cfg(debug_assertions)]
                eprintln!("[dock] icône appliquée : {path}");
            }
            None => {
                #[cfg(debug_assertions)]
                eprintln!("[dock] ÉCHEC chargement .icns : {path}");
            }
        }
    }
}

/// Implémentation `eframe::App` : délègue au module `ui`.
struct ExplorerAppWrapper {
    app: app::ExplorerApp,
    /// macOS : l'icône du Dock est ré-appliquée au premier frame (LaunchServices
    /// écrase celle réglée dans `main()` pendant le lancement).
    #[cfg(target_os = "macos")]
    dock_icon_done: bool,
}

impl eframe::App for ExplorerAppWrapper {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "macos")]
        if !self.dock_icon_done {
            set_dock_icon();
            self.dock_icon_done = true;
        }

        let ctx = ui.ctx().clone();
        ui::show(&mut self.app, &ctx, ui);
    }
}

fn main() -> eframe::Result {
    #[cfg(target_os = "macos")]
    set_dock_icon();

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
                #[cfg(target_os = "macos")]
                dock_icon_done: false,
            }))
        }),
    )
}
