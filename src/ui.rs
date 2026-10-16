//! Rendu de l'interface utilisateur (style Explorateur Windows).

use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, RichText, Vec2};

use crate::app::{Action, Dialog, ExplorerApp, SortColumn, TabKind, ViewMode};
use crate::fs;
use crate::remote;
use sysinfo::{DiskKind, Disks, System};

/// Palette de couleurs façon Windows 11 (Explorateur, thème clair).
mod palette {
    use eframe::egui::Color32;

    /// Fond « Mica » des barres, du volet latéral et de la barre d'état.
    pub const MICA: Color32 = Color32::from_rgb(243, 243, 243);
    pub const TOOLBAR_BG: Color32 = MICA;
    pub const TREE_BG: Color32 = MICA;
    pub const LIST_BG: Color32 = Color32::WHITE;
    pub const STATUS_BG: Color32 = MICA;
    /// Bleu d'accent Windows 11 (#0067C0).
    pub const ACCENT: Color32 = Color32::from_rgb(0, 103, 192);
    /// Bleu d'accent au survol des boutons principaux.
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0, 95, 184);
    /// Sélection d'un élément (bleu clair Windows 11).
    pub const SELECTED: Color32 = Color32::from_rgb(203, 231, 255);
    #[allow(dead_code)]
    pub const SELECTED_INACTIVE: Color32 = Color32::from_rgb(225, 235, 245);
    /// Survol des lignes de la liste.
    pub const HOVER: Color32 = Color32::from_rgb(244, 244, 244);
    /// Survol neutre (pilules, onglets) : noir translucide.
    pub const HOVER_NEUTRAL: Color32 = Color32::from_black_alpha(14);
    /// Survol d'un item de menu contextuel (gris neutre façon Windows 11).
    pub const MENU_HOVER: Color32 = Color32::from_rgb(237, 237, 237);
    /// Bordure fine Windows 11 (#E5E5E5).
    pub const BORDER: Color32 = Color32::from_rgb(229, 229, 229);
    pub const TEXT: Color32 = Color32::from_rgb(27, 27, 27);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(97, 97, 97);
}

/// Icône "dossier" (emoji).
fn folder_icon() -> &'static str {
    "📁"
}

/// Icône selon l'extension.
fn file_icon(ext: &str) -> &'static str {
    match ext {
        "txt" | "md" | "log" | "rtf" => "📄",
        "pdf" => "📕",
        "doc" | "docx" | "odt" => "📘",
        "xls" | "xlsx" | "ods" | "csv" => "📗",
        "ppt" | "pptx" | "odp" => "📙",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" => "🖼️",
        "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" => "🎵",
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" => "🎬",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "🗜️",
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp" | "java" | "go" | "rb" | "php"
        | "html" | "css" | "json" | "xml" | "yml" | "yaml" | "toml" | "sh" => "📝",
        "exe" | "msi" | "app" | "deb" | "rpm" | "dmg" | "bin" => "⚙️",
        "" => "📄",
        _ => "📄",
    }
}

/// Formate une date/heure locale.
fn format_datetime(dt: &chrono::DateTime<chrono::Local>) -> String {
    dt.format("%d/%m/%Y %H:%M").to_string()
}

/// Mesure la largeur rendue d'un texte (police proportionnelle courante).
fn measure_text(ui: &egui::Ui, text: &str, font: egui::FontId) -> f32 {
    ui.ctx().fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font, palette::TEXT)
            .size()
            .x
    })
}

/// Tronque `text` avec « … » pour tenir dans `max_width` pixels.
fn ellipsize(ui: &egui::Ui, text: &str, max_width: f32, font: egui::FontId) -> String {
    if measure_text(ui, text, font.clone()) <= max_width {
        return text.to_string();
    }
    let mut s = text.to_string();
    while s.chars().count() > 1 {
        s.pop();
        if measure_text(ui, &format!("{s}…"), font.clone()) <= max_width {
            break;
        }
    }
    format!("{s}…")
}

/// --- Animations réactives style macOS ---------------------------------------

/// Fonction d'easing « ease-out cubic » (démarrage rapide, fin douce).
/// `t` doit être compris entre 0.0 et 1.0.
fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Interpole linéairement entre deux couleurs.
fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

#[allow(dead_code)]
/// Parse une taille lisible (ex: "10 Mo", "500 Ko", "2.5 Go") en octets.
fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return Some(0);
    }
    let (num_str, unit) = match s.find(|c: char| c.is_alphabetic()) {
        Some(idx) => (s[..idx].trim(), s[idx..].trim()),
        None => (s, "o"),
    };
    let value: f64 = num_str.parse().ok()?;
    let multiplier = match unit {
        "o" | "O" => 1.0,
        "ko" | "Ko" | "kO" | "KB" | "kb" => 1024.0,
        "mo" | "Mo" | "mO" | "MB" | "mb" => 1024.0 * 1024.0,
        "go" | "Go" | "gO" | "GB" | "gb" => 1024.0 * 1024.0 * 1024.0,
        "to" | "To" | "tO" | "TB" | "tb" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((value * multiplier) as u64)
}

/// Couleur selon la taille du fichier : vert (petit) → rouge (gros).
fn file_color(entry: &fs::Entry, max_size: f64) -> Color32 {
    if max_size <= 0.0 {
        return Color32::from_rgb(0, 200, 0); // vert par défaut
    }
    let ratio = (entry.size as f64 / max_size).clamp(0.0, 1.0);
    // Interpolation : vert (0, 255, 0) quand petit, rouge (255, 0, 0) quand gros
    let r = (ratio * 255.0) as u8;
    let g = ((1.0 - ratio) * 255.0) as u8;
    Color32::from_rgb(r, g, 0)
}

/// Retourne true si la couleur est sombre (pour choisir la couleur du texte).
fn is_color_dark(color: Color32) -> bool {
    let r = color.r() as f32;
    let g = color.g() as f32;
    let b = color.b() as f32;
    let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
    luminance < 128.0
}

/// Calcule la valeur d'animation (0.0 → 1.0) pour un élément survolé.
/// `prev` est la valeur précédente, `hovered` l'état actuel, `dt` le delta-temps,
/// `speed` la vitesse de transition (en unités par seconde).
fn hover_anim(prev: f32, hovered: bool, dt: f32, speed: f32) -> f32 {
    let target = if hovered { 1.0 } else { 0.0 };
    let diff = target - prev;
    let step = speed * dt;
    if diff.abs() <= step {
        target
    } else {
        prev + diff.signum() * step
    }
}

/// Style « plat » local : boutons sans fond, surbrillance en pilule grise

/// Informations système collectées pour le popup « Propriétés de l'ordinateur ».
struct SystemInfo {
    os_name: String,
    os_version: String,
    os_code_name: String,
    cpu_brand: String,
    cpu_cores: String,
    cpu_freq_mhz: u64,
    ram_total: u64,
    ram_available: u64,
    ram_used: u64,
    disks: Vec<(String, u64, u64, String)>,
}

/// Style « plat » local : boutons sans fond, surbrillance en pilule grise
/// (barre de commandes Windows 11).
fn flat_style(base: &egui::Style) -> egui::Style {
    let mut style = base.clone();
    style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(0.0, Color32::TRANSPARENT);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.hovered.weak_bg_fill = palette::MENU_HOVER;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(0.0, Color32::TRANSPARENT);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette::TEXT);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
    style.visuals.widgets.active.weak_bg_fill = palette::BORDER;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(0.0, Color32::TRANSPARENT);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);
    style
}

/// Bouton de menu « plat » : fond transparent, pilule grise au survol.
fn flat_menu_button(ui: &mut egui::Ui, title: RichText, add: impl FnOnce(&mut egui::Ui)) {
    let style = flat_style(ui.style());
    ui.scope(|ui| {
        ui.set_style(style);
        ui.menu_button(title, add);
    });
}

/// Sélecteur de mode d'affichage : boutons « Liste », « Détail » et
/// « Miniatures » regroupés à droite de la barre de commandes (masqué sur les
/// fenêtres étroites : le menu « … » reste accessible).
fn view_mode_buttons(app: &mut ExplorerApp, ui: &mut egui::Ui) {
    if ui.available_width() < 230.0 {
        return;
    }
    ui.horizontal(|ui| {
        for (mode, label) in [
            (ViewMode::List, "Liste"),
            (ViewMode::Details, "Détail"),
            (ViewMode::Thumbnails, "Miniatures"),
        ] {
            let selected = app.tab().view_mode == mode;
            let tip = match mode {
                ViewMode::List => "Vue en liste (Ctrl+1)",
                ViewMode::Details => "Vue en détail (Ctrl+2)",
                ViewMode::Thumbnails => "Vue en miniatures (Ctrl+3)",
            };
            let resp = ui
                .add(egui::Button::new(RichText::new(label).size(13.0)).selected(selected))
                .on_hover_text(tip);
            if resp.clicked() {
                app.tab_mut().view_mode = mode;
            }
        }
    });
}

/// Barre de commandes style Windows 11 : « Nouveau ▾ », actions sur la
/// sélection (icônes), « Liste / Détail / Miniatures », puis « Trier ▾ »,
/// « Afficher ▾ » et « … » à droite.
fn command_bar(app: &mut ExplorerApp, ui: &mut egui::Ui) -> Action {
    let mut action = Action::None;
    let has_sel = !app.tab().selected.is_empty();

    ui.horizontal(|ui| {
        ui.add_space(4.0);

        // --- Groupe distant / déverrouillage du magasin d'identifiants ---
        if app.is_remote() {
            if let Some(s) = app.tab().remote_session.as_ref() {
                ui.label(
                    RichText::new("Distant : ")
                        .strong()
                        .size(13.0)
                        .color(palette::ACCENT),
                );
                ui.label(format!(
                    "{} {}",
                    s.config.protocol,
                    s.config.display_address()
                ))
                .on_hover_text("Connexion distante active");
                if icon_button(ui, "🔌", "Déconnecter", true) {
                    app.disconnect_remote();
                }
            }
        } else if app.master_password.is_none() {
            if icon_button(ui, "🔒", "Déverrouiller le magasin d'identifiants", true) {
                app.unlock_dialog.open = true;
            }
        } else {
            if icon_button(ui, "🖧", "Se connecter à un serveur distant", true) {
                app.remote_dialog.open = true;
            }
        }
        if app.is_remote() {
            ui.add_space(8.0);
        } else {
            ui.add_space(8.0);
            vseparator(ui);
            ui.add_space(8.0);
        }

        // --- Nouveau ▾ (sous-menu) ---
        if !app.is_remote() {
            flat_menu_button(ui, RichText::new("➕  Nouveau  ▾").size(14.0), |ui| {
                ui.set_width(MENU_WIDTH);
                if menu_item(ui, "📁", "Dossier", "", false, true) {
                    action = Action::NewFolder;
                    ui.close();
                }
                if menu_item(ui, "📄", "Document texte", "", false, true) {
                    action = Action::NewFile;
                    ui.close();
                }
            });
            ui.add_space(6.0);
            vseparator(ui);
            ui.add_space(6.0);
        }

        // --- Actions sur la sélection (icônes, comme Windows 11) ---
        if icon_button(ui, "✂️", "Couper (Ctrl+X)", has_sel) {
            action = Action::Cut;
        }
        if icon_button(ui, "📋", "Copier (Ctrl+C)", has_sel) {
            action = Action::Copy;
        }
        if icon_button(
            ui,
            "📥",
            "Coller (Ctrl+V)",
            app.clipboard.is_some() && !app.is_remote(),
        ) {
            action = Action::Paste;
        }
        if icon_button(ui, "✏️", "Renommer (F2)", app.tab().selected.len() == 1) {
            action = Action::Rename;
        }
        if icon_button(ui, "🗑️", "Supprimer (Suppr)", has_sel) {
            action = Action::Delete;
        }

        // --- Bouton carte des tailles de fichiers ---
        let fsm_id = ui.id().with("file_size_map_button");
        let fsm_rect = ui.available_rect_before_wrap();
        let mouse_pos = ui.ctx().input(|i| i.pointer.hover_pos());
        let fsm_hovered = mouse_pos.is_some_and(|p| fsm_rect.contains(p));
        let fsm_anim = {
            let prev = ui
                .ctx()
                .memory_mut(|m| m.data.get_temp::<f32>(fsm_id))
                .unwrap_or(0.0);
            let dt = ui.ctx().input(|i| i.predicted_dt.max(1.0 / 60.0));
            let anim = hover_anim(prev, fsm_hovered, dt, 5.0);
            ui.ctx().memory_mut(|m| m.data.insert_temp(fsm_id, anim));
            anim
        };
        if fsm_anim > 0.001 || fsm_anim < 0.999 {
            ui.ctx().request_repaint();
        }
        let fsm_bg = lerp_color(
            Color32::TRANSPARENT,
            palette::HOVER_NEUTRAL,
            ease_out_cubic(fsm_anim),
        );
        ui.painter()
            .rect_filled(fsm_rect, egui::CornerRadius::same(6), fsm_bg);

        let fsm_resp = ui.add(
            egui::Button::new(
                RichText::new("📊 Taille des fichiers")
                    .strong()
                    .size(12.0)
                    .color(palette::TEXT_DIM),
            )
            .frame(false),
        );
        if fsm_resp.clicked() {
            // Filtre par défaut : 0 (tous les fichiers) - ajustez avec le champ ci-dessous
            app.dialog = Dialog::FileSizeMap { min_size: 0 };
        }

        // --- Groupe de droite : Trier, Afficher, menu « … » ---
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);
            // Menu « … » (commandes secondaires, comme « See more » sous Windows).
            flat_menu_button(ui, RichText::new("⋯").size(18.0), |ui| {
                ui.set_width(MENU_WIDTH);
                if menu_item(ui, "🔲", "Tout sélectionner", "Ctrl+A", false, true) {
                    app.select_all();
                    ui.close();
                }
                if menu_item(ui, "🔄", "Actualiser", "F5", false, true) {
                    action = Action::Refresh;
                    ui.close();
                }
                if menu_item(ui, "ℹ️", "Propriétés du dossier", "", false, true) {
                    let path = app.tab().cwd.clone();
                    app.dialog = Dialog::Properties { path };
                    ui.close();
                }
                menu_separator(ui);
                // Modes d'affichage aussi dans « … » : accessibles sur
                // fenêtre étroite quand les boutons sont masqués.
                for (mode, label) in [
                    (ViewMode::List, "Liste"),
                    (ViewMode::Details, "Détail"),
                    (ViewMode::Thumbnails, "Miniatures"),
                ] {
                    let selected = app.tab().view_mode == mode;
                    if menu_item(ui, if selected { "✓" } else { "" }, label, "", false, true) {
                        app.tab_mut().view_mode = mode;
                        ui.close();
                    }
                }
                menu_separator(ui);
                if menu_item(ui, "❓", "À propos de SmartExplorer", "", false, true) {
                    app.dialog = Dialog::About;
                    ui.close();
                }
                if menu_item(ui, "❌", "Quitter", "Alt+F4", false, true) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close();
                }
            });
            ui.add_space(2.0);
            // Afficher ▾ : fichiers cachés.
            flat_menu_button(
                ui,
                RichText::new("👁️  Afficher  ▾").size(14.0),
                |ui| {
                    ui.set_width(MENU_WIDTH);
                    let on = app.tab().show_hidden;
                    if menu_item(
                        ui,
                        if on { "✓" } else { "" },
                        "Fichiers cachés",
                        "",
                        false,
                        true,
                    ) {
                        action = Action::ToggleHidden;
                        ui.close();
                    }
                },
            );
            ui.add_space(2.0);
            // Trier ▾ (la coche suit la colonne courante).
            flat_menu_button(ui, RichText::new("↕  Trier  ▾").size(14.0), |ui| {
                ui.set_width(MENU_WIDTH);
                for (col, label) in [
                    (SortColumn::Name, "Nom"),
                    (SortColumn::Type, "Type"),
                    (SortColumn::Size, "Taille"),
                    (SortColumn::Modified, "Modifié le"),
                ] {
                    let current = app.tab().sort_column == col;
                    if menu_item(ui, if current { "✓" } else { "" }, label, "", false, true) {
                        app.toggle_sort(col);
                        action = Action::Refresh;
                        ui.close();
                    }
                }
            });
            // --- Mode d'affichage : Liste / Détail / Miniatures ---
            ui.add_space(2.0);
            vseparator(ui);
            ui.add_space(2.0);
            view_mode_buttons(app, ui);
        });
    });

    action
}

/// Barre d'adresse style Windows 11 : boutons ←/→/↑, fil d'Ariane dans une
/// pilule blanche puis recherche dans une pilule séparée à droite.
fn address_bar(app: &mut ExplorerApp, ui: &mut egui::Ui) -> Action {
    let mut action = Action::None;

    ui.horizontal(|ui| {
        ui.add_space(2.0);

        // --- Navigation ---
        if icon_button(
            ui,
            "←",
            "Précédent (Alt+←)",
            app.can_back() && !app.is_remote(),
        ) {
            action = Action::Back;
        }
        if icon_button(
            ui,
            "→",
            "Suivant (Alt+→)",
            app.can_forward() && !app.is_remote(),
        ) {
            action = Action::Forward;
        }
        if icon_button(ui, "↑", "Dossier parent (Retour arrière)", !app.is_remote()) {
            action = Action::Up;
        }
        ui.add_space(4.0);

        // Largeur de la pilule de recherche ; le fil d'Ariane prend le reste.
        let search_w = 260.0;
        let crumb_w = (ui.available_width() - search_w - 12.0).max(140.0);

        // --- Fil d'Ariane (pilule blanche) ---
        egui::Frame::NONE
            .fill(Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, palette::BORDER))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(crumb_w - 16.0, 26.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        if app.is_remote() {
                            if let Some(s) = app.tab().remote_session.as_ref() {
                                ui.label(
                                    RichText::new(format!(
                                        "{} {} › {}",
                                        s.config.protocol,
                                        s.config.display_address(),
                                        s.cwd
                                    ))
                                    .size(13.0),
                                );
                            }
                        } else {
                            let ancestors: Vec<PathBuf> = {
                                let mut v: Vec<PathBuf> =
                                    app.tab().cwd.ancestors().map(|p| p.to_path_buf()).collect();
                                v.reverse(); // de la racine vers le dossier courant
                                v
                            };
                            for (i, part) in ancestors.iter().enumerate() {
                                let label = if i == 0 {
                                    if cfg!(windows) {
                                        part.to_string_lossy().to_string()
                                    } else {
                                        "/".to_string()
                                    }
                                } else {
                                    part.file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default()
                                };
                                if crumb_button(ui, &label) {
                                    action = Action::Navigate(part.clone());
                                }
                                if i + 1 < ancestors.len() {
                                    ui.add_space(1.0);
                                    ui.label(
                                        RichText::new("›").size(13.0).color(palette::TEXT_DIM),
                                    );
                                    ui.add_space(1.0);
                                }
                            }
                        }
                    },
                );
            });

        // --- Recherche (pilule séparée, comme sous Windows 11) ---
        let folder_name = app
            .tab()
            .cwd
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| app.tab().cwd.to_string_lossy().to_string());
        egui::Frame::NONE
            .fill(Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, palette::BORDER))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔎").size(13.0).color(palette::TEXT_DIM));
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut app.tab_mut().search)
                            .hint_text(format!("Rechercher dans {folder_name}"))
                            .desired_width(search_w - 78.0),
                    );
                    if resp.changed() {
                        app.refresh();
                    }
                    // Portée de la recherche (comme « Rechercher dans » de Windows).
                    ui.menu_button(
                        RichText::new("▾").size(11.0).color(palette::TEXT_DIM),
                        |ui| {
                            ui.set_width(260.0);
                            if ui
                                .checkbox(
                                    &mut app.tab_mut().search_recursive,
                                    "Dans les sous-dossiers",
                                )
                                .changed()
                            {
                                app.refresh();
                            }
                        },
                    );
                });
            });
    });

    action
}

/// Bouton icône carré (32×32), coins 4 px, pilule grise au survol
/// (barre d'outils Windows 11). Gris clair et inactif si `enabled` est faux.
fn icon_button(ui: &mut egui::Ui, icon: &str, tooltip: &str, enabled: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
    let resp = resp.on_hover_text(tooltip);
    if enabled && (resp.hovered() || resp.is_pointer_button_down_on()) {
        let fill = if resp.is_pointer_button_down_on() {
            palette::BORDER
        } else {
            palette::MENU_HOVER
        };
        ui.painter().rect_filled(rect, 4.0, fill);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(15.0),
        if enabled {
            palette::TEXT
        } else {
            Color32::from_gray(175)
        },
    );
    enabled && resp.clicked()
}

/// Séparateur vertical fin entre groupes de commandes.
fn vseparator(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 22.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, palette::BORDER);
}

/// Bouton principal (fond bleu d'accent) des dialogues Windows 11.
fn primary_button(ui: &mut egui::Ui, label: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(96.0, 32.0), egui::Sense::click());
    let fill = if resp.is_pointer_button_down_on() {
        Color32::from_rgb(0, 80, 152)
    } else if resp.hovered() {
        palette::ACCENT_HOVER
    } else {
        palette::ACCENT
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(14.0),
        Color32::WHITE,
    );
    resp.clicked()
}

/// Élément du panneau latéral : icône + libellé, pilule pleine largeur à la
/// sélection (bleu clair) ou au survol (gris neutre), façon Windows 11.
/// `texture` : icône système réelle (macOS) en 16 px ; à défaut, emoji `icon`.
fn nav_item(
    ui: &mut egui::Ui,
    texture: Option<&egui::TextureHandle>,
    icon: &str,
    label: &str,
    selected: bool,
) -> bool {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::click());
    if selected {
        ui.painter().rect_filled(rect, 4.0, palette::SELECTED);
    } else if resp.hovered() || resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, 4.0, palette::HOVER_NEUTRAL);
    }
    let mid = rect.center().y;
    match texture {
        Some(tex) => {
            // Icône système réelle, centrée sur la colonne d'icône (10..26).
            let size = egui::vec2(16.0, 16.0);
            ui.painter().image(
                tex.id(),
                egui::Rect::from_center_size(egui::pos2(rect.left() + 18.0, mid), size),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => {
            ui.painter().text(
                egui::pos2(rect.left() + 10.0, mid),
                egui::Align2::LEFT_CENTER,
                icon,
                egui::FontId::proportional(14.0),
                palette::TEXT,
            );
        }
    }
    ui.painter().text(
        egui::pos2(rect.left() + 34.0, mid),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        palette::TEXT,
    );
    resp.clicked()
}

/// Segment cliquable du fil d'Ariane (pilule blanche de la barre d'adresse).
fn crumb_button(ui: &mut egui::Ui, label: &str) -> bool {
    let font = egui::FontId::proportional(14.0);
    let width = measure_text(ui, label, font.clone()) + 14.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::click());
    if resp.hovered() || resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, 4.0, palette::MENU_HOVER);
    }
    ui.painter().text(
        egui::pos2(rect.left() + 7.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        palette::TEXT,
    );
    resp.clicked()
}

/// Panneau latéral : accès rapides + arborescence des dossiers.
fn tree_panel(app: &mut ExplorerApp, ui: &mut egui::Ui, ctx: &egui::Context) -> Action {
    let mut action = Action::None;

    // Accès rapides (masqués sur un serveur distant).
    if !app.is_remote() {
        ui.add_space(4.0);
        ui.label(
            RichText::new("Accès rapide")
                .strong()
                .size(13.0)
                .color(palette::TEXT_DIM),
        );
        ui.add_space(4.0);

        if let Some(home) = fs::home_dir() {
            let sel = app.tab().cwd == home;
            let texture = app.icons.system(ui.ctx(), &home, 16);
            if nav_item(ui, texture.as_ref(), "🏠", &user_home_label(&home), sel) {
                action = Action::Navigate(home);
            }
        }
        // Raccourcis standards via `dirs`.
        for (icon, opt, label) in [
            ("🖥️", dirs::desktop_dir(), "Bureau"),
            ("📥", dirs::download_dir(), "Téléchargements"),
            ("📄", dirs::document_dir(), "Documents"),
            ("🖼️", dirs::picture_dir(), "Images"),
            ("🎵", dirs::audio_dir(), "Musique"),
            ("🎬", dirs::video_dir(), "Vidéos"),
        ] {
            if let Some(path) = opt
                && path.exists()
            {
                let texture = app.icons.system(ui.ctx(), &path, 16);
                if nav_item(ui, texture.as_ref(), icon, label, app.tab().cwd == path) {
                    action = Action::Navigate(path);
                }
            }
        }
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);

    // Bouton « Ce PC » avec survol animé (style macOS) et menu contextuel.
    let ce_pc_id = ui.id().with("ce_pc_button");
    let ce_pc_rect = ui.available_rect_before_wrap();
    let mouse_pos = ui.ctx().input(|i| i.pointer.hover_pos());
    let ce_pc_hovered = mouse_pos.is_some_and(|p| ce_pc_rect.contains(p));
    let ce_pc_anim = {
        let prev = ctx
            .memory_mut(|m| m.data.get_temp::<f32>(ce_pc_id))
            .unwrap_or(0.0);
        let dt = ui.ctx().input(|i| i.predicted_dt.max(1.0 / 60.0));
        let anim = hover_anim(prev, ce_pc_hovered, dt, 5.0);
        ctx.memory_mut(|m| m.data.insert_temp(ce_pc_id, anim));
        anim
    };
    if ce_pc_anim > 0.001 || ce_pc_anim < 0.999 {
        ctx.request_repaint();
    }

    // Fond arrondi qui s'illumine au survol.
    let ce_pc_bg = lerp_color(
        Color32::TRANSPARENT,
        palette::HOVER_NEUTRAL,
        ease_out_cubic(ce_pc_anim),
    );
    ui.painter()
        .rect_filled(ce_pc_rect, egui::CornerRadius::same(6), ce_pc_bg);

    let ce_pc_resp = ui.add(
        egui::Button::new(
            RichText::new("Ce PC")
                .strong()
                .size(13.0)
                .color(palette::TEXT_DIM),
        )
        .frame(false),
    );
    ce_pc_resp.context_menu(|ui| {
        ui.set_width(200.0);
        if menu_item(ui, "ℹ️", "Propriétés", "", false, true) {
            app.dialog = Dialog::SystemProperties;
            ui.close();
        }
    });
    ui.add_space(4.0);

    // Racines (lecteurs / système) ou fichiers du serveur distant.
    if app.is_remote() {
        render_remote_ce_pc(app, ui, ctx);
    } else {
        for root in fs::get_roots() {
            let label = if cfg!(windows) {
                root.to_string_lossy().to_string()
            } else {
                "Ordinateur".to_string()
            };
            tree_node(app, &root, label, ui, &mut action);
        }
    }

    action
}

/// Libellé du home (nom du dossier utilisateur).
fn user_home_label(home: &std::path::Path) -> String {
    home.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Utilisateur".to_string())
}

/// Nœud récursif de l'arborescence (dépliable).
fn tree_node(
    app: &mut ExplorerApp,
    path: &std::path::Path,
    label: String,
    ui: &mut egui::Ui,
    action: &mut Action,
) {
    let path_buf = path.to_path_buf();
    let is_open = app.tab().expanded.contains(&path_buf);
    let subdirs = fs::list_subdirs(path);
    let has_children = !subdirs.is_empty();

    // En-tête : flèche de dépliage + libellé cliquable (pilule Windows 11).
    ui.horizontal(|ui| {
        if has_children {
            let r = ui.add(
                egui::Button::new(if is_open { "▾" } else { "▸" })
                    .frame(false)
                    .min_size(Vec2::new(16.0, 16.0)),
            );
            if r.clicked() {
                if is_open {
                    app.tab_mut().expanded.remove(&path_buf);
                } else {
                    app.tab_mut().expanded.insert(path_buf.clone());
                }
            }
        } else {
            ui.add_sized(
                Vec2::new(16.0, 16.0),
                egui::Label::new(RichText::new("•").color(palette::TEXT_DIM)),
            );
        }

        let name = if path.parent().is_none() {
            label.clone()
        } else {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| label.clone())
        };
        let selected = app.tab().cwd == path_buf;
        let texture = app.icons.system(ui.ctx(), &path_buf, 16);
        if nav_item(ui, texture.as_ref(), folder_icon(), &name, selected) {
            *action = Action::Navigate(path_buf.clone());
        }
    });

    // Enfants (si déplié).
    if is_open && has_children {
        ui.indent(path_buf.clone(), |ui| {
            for sub in subdirs {
                let sub_label = sub
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                tree_node(app, &sub, sub_label, ui, action);
            }
        });
    }
}

/// Affiche sous « Ce PC » les fichiers du serveur distant connecté.
/// Chaque entrée est cliquable : un dossier navigue, un fichier se lit.
fn render_remote_ce_pc(app: &mut ExplorerApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    // On extrait les données détenues avant toute mutation de `app` (navigation
    // distante / lecture de fichier), afin d'éviter tout conflit d'emprunt.
    let (entries, cwd) = match app.tab().remote_session.as_ref() {
        Some(s) => (s.entries.clone(), s.cwd.clone()),
        None => return,
    };

    if entries.is_empty() {
        ui.label(
            RichText::new("(aucun fichier)")
                .color(palette::TEXT_DIM)
                .italics(),
        );
        return;
    }

    for entry in &entries {
        let remote_path = format!("{}/{}", cwd.trim_end_matches('/'), entry.name);
        let is_dir = entry.is_dir;
        let texture = app.icons.system(ui.ctx(), &PathBuf::from(&remote_path), 16);
        let icon = if is_dir {
            folder_icon()
        } else {
            file_icon(entry.name.rsplit('.').next().unwrap_or(""))
        };
        let selected = app.tab().cwd == Path::new(&remote_path);
        if nav_item(ui, texture.as_ref(), icon, &entry.name, selected) {
            if is_dir {
                app.navigate_remote(&remote_path);
            } else {
                app.read_remote_file(remote_path.clone(), ctx);
            }
        }
    }
}

/// Liste principale des fichiers : bandeau d'erreur éventuel, puis rendu
/// selon le mode d'affichage courant (Détail / Liste / Miniatures).
fn file_list(app: &mut ExplorerApp, ui: &mut egui::Ui) -> Action {
    let mut action = Action::None;

    // Message d'erreur éventuel.
    if let Some(err) = app.tab().error.clone() {
        egui::Frame::NONE
            .fill(Color32::from_rgb(255, 235, 235))
            .inner_margin(6.0)
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠️").size(16.0));
                    ui.label(RichText::new(err).color(Color32::from_rgb(160, 32, 32)));
                });
            });
        ui.add_space(4.0);
    }

    // --- Choix de la vue (Ctrl+1/2/3 et sélecteur de la barre de commandes) ---
    let modifiers = ui.input(|i| i.modifiers);
    let ctrl = modifiers.ctrl || modifiers.command;
    let shift = modifiers.shift;
    match app.tab().view_mode {
        ViewMode::Details => details_view(app, ui, ctrl, shift, &mut action),
        ViewMode::List if app.tab().entries.is_empty() => show_empty_folder(ui),
        ViewMode::Thumbnails if app.tab().entries.is_empty() => show_empty_folder(ui),
        ViewMode::List => list_view(app, ui, ctrl, shift, &mut action),
        ViewMode::Thumbnails => grid_view(app, ui, ctrl, shift, &mut action),
    }
    action
}

/// Vue « Détail » : en-têtes de colonnes cliquables (le tri), puis les
/// lignes du dossier — le tableau historique de SmartExplorer.
fn details_view(
    app: &mut ExplorerApp,
    ui: &mut egui::Ui,
    ctrl: bool,
    shift: bool,
    action: &mut Action,
) {
    // --- En-têtes de colonnes (cliquables pour le tri) ---
    // Même margine que les lignes (PAD) : chaque colonne d'en-tête partage
    // exactement la position X de sa colonne de données.
    let col_widths = ColumnWidths::new(ui);
    egui::Frame::NONE
        .inner_margin(egui::Margin {
            left: ColumnWidths::PAD as i8,
            right: ColumnWidths::PAD as i8,
            top: 0,
            bottom: 0,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                header_cell(
                    ui,
                    "Nom",
                    col_widths.name,
                    SortColumn::Name,
                    false,
                    app,
                    action,
                );
                header_cell(
                    ui,
                    "Type",
                    col_widths.type_col,
                    SortColumn::Type,
                    false,
                    app,
                    action,
                );
                // « Taille » : en-tête à droite, comme les valeurs (Windows).
                header_cell(
                    ui,
                    "Taille",
                    col_widths.size,
                    SortColumn::Size,
                    true,
                    app,
                    action,
                );
                header_cell(
                    ui,
                    "Modifié le",
                    col_widths.modified,
                    SortColumn::Modified,
                    false,
                    app,
                    action,
                );
            });
        });
    ui.separator();

    // --- Lignes ---
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if app.tab().entries.is_empty() {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Ce dossier est vide.").color(palette::TEXT_DIM));
                });
                return;
            }

            // Fourchette d'index : l'état peut changer pendant le rendu
            // (renommage inline) ; le garde-fou évite un panic d'indexation.
            for index in 0..app.tab().entries.len() {
                if index >= app.tab().entries.len() {
                    break;
                }
                let row_response = render_row(app, ui, index, col_widths, ctrl, shift, action);
                let _ = row_response;
            }
        });
}

/// Largeurs des colonnes du tableau.
#[derive(Clone, Copy)]
struct ColumnWidths {
    name: f32,
    type_col: f32,
    size: f32,
    modified: f32,
}

impl ColumnWidths {
    /// Marge gauche/droite du contenu dans la carte (partagée par l'en-tête
    /// et par les lignes pour un alignement exact des colonnes).
    const PAD: f32 = 6.0;

    fn new(ui: &egui::Ui) -> Self {
        // Retrait des marges et des espacements inter-colonnes (3 gaps),
        // pour que cells + gaps + marges = largeur exacte de la carte.
        let gap = ui.spacing().item_spacing.x;
        let total = ui.available_width() - Self::PAD * 2.0 - gap * 3.0;
        // Répartition : nom = reste, type/taille/date fixes.
        let type_col = 130.0;
        let size = 90.0;
        let modified = 150.0;
        let name = (total - type_col - size - modified).max(80.0);
        ColumnWidths {
            name,
            type_col,
            size,
            modified,
        }
    }
}

/// Cellule d'en-tête cliquable (déclenche le tri) : survol arrondi, colonne
/// active en gras avec flèche ▲/▼.
fn header_cell(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    col: SortColumn,
    right: bool,
    app: &mut ExplorerApp,
    action: &mut Action,
) {
    let active = app.tab().sort_column == col;
    let arrow = if active {
        match app.tab().sort_order {
            crate::app::SortOrder::Asc => " ▲",
            crate::app::SortOrder::Desc => " ▼",
        }
    } else {
        ""
    };
    let text = format!("{label}{arrow}");
    let font = egui::FontId::proportional(13.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::click());
    if resp.hovered() || resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, 4.0, palette::MENU_HOVER);
    }
    // Calé sur les données : aucun retrait interne, « Taille » alignée à
    // droite (comme Windows) et les autres colonnes à gauche.
    let (pos, align) = if right {
        (
            egui::pos2(rect.right(), rect.center().y),
            egui::Align2::RIGHT_CENTER,
        )
    } else {
        (
            egui::pos2(rect.left(), rect.center().y),
            egui::Align2::LEFT_CENTER,
        )
    };
    ui.painter()
        .text(pos, align, text.as_str(), font.clone(), palette::TEXT);
    if active {
        // Effet « gras » : seconde impression légèrement décalée.
        ui.painter().text(
            pos + egui::vec2(0.5, 0.0),
            align,
            text.as_str(),
            font,
            palette::TEXT,
        );
    }
    if resp.clicked() {
        app.toggle_sort(col);
        *action = Action::Refresh;
    }
}

/// Rend une ligne du tableau. Retourne la réponse globale de la ligne.
fn render_row(
    app: &mut ExplorerApp,
    ui: &mut egui::Ui,
    index: usize,
    widths: ColumnWidths,
    ctrl: bool,
    shift: bool,
    action: &mut Action,
) -> egui::Response {
    // On copie les données nécessaires pour éviter d'emprunter `app` pendant le rendu.
    let entry = app.tab().entries[index].clone();
    let selected = app.tab().selected.contains(&entry.path);
    let renaming = app.tab().renaming.as_ref() == Some(&entry.path);

    // Rectangle de la ligne : cells + espacements inter-colonnes + marges,
    // donc la même emprise exacte que l'en-tête (colonnes alignées).
    let gap = ui.spacing().item_spacing.x;
    let content_w = widths.name + widths.type_col + widths.size + widths.modified + gap * 3.0;
    let row_rect = egui::Rect::from_min_size(
        ui.available_rect_before_wrap().min,
        egui::vec2(content_w + ColumnWidths::PAD * 2.0, 34.0),
    );
    // Fond de la ligne : sélection (bleu clair) ou survol (gris neutre).
    let bg = if selected {
        palette::SELECTED
    } else if ui.rect_contains_pointer(row_rect) {
        palette::HOVER
    } else {
        Color32::TRANSPARENT
    };

    let row = egui::Frame::NONE
        .fill(bg)
        .corner_radius(4.0)
        .inner_margin(egui::Margin {
            left: ColumnWidths::PAD as i8,
            right: ColumnWidths::PAD as i8,
            top: 4,
            bottom: 4,
        })
        .show(ui, |ui| {
            ui.set_min_width(content_w);
            ui.horizontal(|ui| {
                // --- Colonne Nom (icône + nom ou champ de renommage) ---
                // Cellule = réserve EXACTE (comme l'en-tête) + Ui enfant au
                // même rect : `allocate_ui_with_layout` n'avance le curseur
                // que de la largeur peinte et décalait les colonnes suivantes.
                let (name_rect, _) =
                    ui.allocate_exact_size(egui::vec2(widths.name, 26.0), egui::Sense::hover());
                let mut name_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(name_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                if renaming {
                    let te = name_ui.add(
                        egui::TextEdit::singleline(&mut app.tab_mut().rename_buffer)
                            .desired_width(widths.name - 8.0),
                    );
                    if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let path = entry.path.clone();
                        let name = app.tab().rename_buffer.clone();
                        app.rename_entry(&path, &name);
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        app.tab_mut().renaming = None;
                    }
                } else {
                    // Indicateur de lien symbolique.
                    let link_mark = if entry.is_symlink { "↗" } else { "" };
                    let color = if entry.is_hidden {
                        palette::TEXT_DIM
                    } else {
                        palette::TEXT
                    };

                    // Icône système réelle (macOS) ; emoji de repli sinon.
                    add_row_icon(app, &mut name_ui, &entry, 16.0);
                    name_ui.add(
                        egui::Label::new(
                            RichText::new(format!("{}{link_mark}", entry.name)).color(color),
                        )
                        .selectable(false),
                    );
                }
                // --- Colonne Type ---
                let (type_rect, _) =
                    ui.allocate_exact_size(egui::vec2(widths.type_col, 26.0), egui::Sense::hover());
                let mut type_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(type_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                type_ui.label(RichText::new(entry.type_label()).color(palette::TEXT_DIM));

                // --- Colonne Taille (alignée à droite, sous l'en-tête) ---
                let (size_rect, _) =
                    ui.allocate_exact_size(egui::vec2(widths.size, 26.0), egui::Sense::hover());
                let mut size_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(size_rect)
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                let text = if entry.is_dir {
                    String::new()
                } else {
                    fs::format_size(entry.size)
                };
                size_ui.label(RichText::new(text).color(palette::TEXT_DIM));

                // --- Colonne Date ---
                let (date_rect, _) =
                    ui.allocate_exact_size(egui::vec2(widths.modified, 26.0), egui::Sense::hover());
                let mut date_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(date_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                let text = entry
                    .modified
                    .map(|dt| format_datetime(&dt))
                    .unwrap_or_default();
                date_ui.label(RichText::new(text).color(palette::TEXT_DIM));
            });
            ui.response()
        });

    // --- Flash animé vert sur l'élément concerné par une action récente ---
    if let Some(flash) = row_flash(app, &entry.path) {
        ui.painter().rect_filled(
            row.response.rect,
            8.0,
            Color32::from_rgba_unmultiplied(46, 165, 90, (130.0 * flash) as u8),
        );
        // L'animation demande un rafraîchissement continu.
        ui.ctx().request_repaint();
    }

    // Interaction sur toute la largeur de la ligne.
    let full_resp = ui.interact(row.response.rect, ui.id().with(index), egui::Sense::click());

    // Clic simple : sélection.
    if full_resp.clicked() {
        app.handle_click(index, ctrl, shift);
    }

    // Double-clic : ouvrir. L'action est différée : appeler `open_entry`
    // pendant le rendu remplacerait `app.tab().entries` et l'application se
    // fermerait (panic d'indexation) si le dossier cible est plus court.
    if full_resp.double_clicked() {
        let path = entry.path.clone();
        *action = Action::Open(path);
    }

    // Survol : changer le curseur.
    if full_resp.hovered() && entry.is_dir {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    // Menu contextuel (clic droit).
    full_resp.context_menu(|ui| {
        context_menu(app, ui, &entry, index, action);
    });

    full_resp
}

/// Vue « Liste » : rangées de tuiles à colonnes calculées (icône + nom
/// tronqué), façon Explorateur Windows en vue « Liste ». Contrairement à un
/// `horizontal_wrapped`, chaque tuile occupe une réserve exacte : survol,
/// sélection et clic restent toujours alignés (pas de tuile fantôme en fin
/// de ligne), et les rangées font exactement `TILE_H` pour `show_rows`.
fn list_view(
    app: &mut ExplorerApp,
    ui: &mut egui::Ui,
    ctrl: bool,
    shift: bool,
    action: &mut Action,
) {
    const TILE_W: f32 = 158.0;
    const TILE_H: f32 = 30.0;
    const GAP: f32 = 6.0;

    let total = app.tab().entries.len();
    let cols = (((ui.available_width() + GAP) / (TILE_W + GAP)).floor() as usize).max(1);
    let rows = total.div_ceil(cols);

    // L'écart vertical entre les rangées DOIT valoir GAP : `show_rows` s'en
    // sert pour placer la fenêtre de troncature et la hauteur totale.
    ui.spacing_mut().item_spacing.y = GAP;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, TILE_H, rows, |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    let start = row * cols;
                    for index in start..(start + cols).min(total) {
                        list_tile(app, ui, index, TILE_W, TILE_H, ctrl, shift, action);
                    }
                });
            }
        });
}

/// Une tuile de la vue Liste : réserve exacte, fond (sélection / survol)
/// peint au rect, contenu dans un enfant ui au même rect (icône 16 px puis
/// nom tronqué — ou renommage inline), puis interactions communes.
#[allow(clippy::too_many_arguments)]
fn list_tile(
    app: &mut ExplorerApp,
    ui: &mut egui::Ui,
    index: usize,
    tile_w: f32,
    tile_h: f32,
    ctrl: bool,
    shift: bool,
    action: &mut Action,
) {
    if index >= app.tab().entries.len() {
        return;
    }
    let entry = app.tab().entries[index].clone();
    let selected = app.tab().selected.contains(&entry.path);
    let renaming = app.tab().renaming.as_ref() == Some(&entry.path);

    // Réserve EXACTE : la position de la tuile est sans ambiguïté.
    let (rect, _) = ui.allocate_exact_size(egui::vec2(tile_w, tile_h), egui::Sense::hover());
    let bg = if selected {
        palette::SELECTED
    } else if ui.rect_contains_pointer(rect) {
        palette::HOVER_NEUTRAL
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 4.0, bg);

    // Contenu dans le rect exact (peint par-dessus le fond).
    let mut tile_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(egui::vec2(6.0, 4.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    if renaming {
        let te = tile_ui.add(
            egui::TextEdit::singleline(&mut app.tab_mut().rename_buffer)
                .desired_width(tile_w - 44.0),
        );
        if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let path = entry.path.clone();
            let name = app.tab().rename_buffer.clone();
            app.rename_entry(&path, &name);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.tab_mut().renaming = None;
        }
    } else {
        add_row_icon(app, &mut tile_ui, &entry, 16.0);
        let color = if entry.is_hidden {
            palette::TEXT_DIM
        } else {
            palette::TEXT
        };
        // Budget : intérieur de la tuile moins icône et espacement.
        let budget = tile_w - 12.0 - 22.0;
        let text = ellipsize(ui, &entry.name, budget, egui::FontId::proportional(13.0));
        tile_ui.label(RichText::new(text).color(color));
    }
    tile_response(app, ui, rect, index, ctrl, shift, action);
}

/// Vue « Miniatures » : grille de vignettes (aperçu réel des images, sinon
/// icône système agrandie) avec le nom dessous ; troncature virtuelle via
/// `show_rows` (une seule rangée peinte à la fois).
fn grid_view(
    app: &mut ExplorerApp,
    ui: &mut egui::Ui,
    ctrl: bool,
    shift: bool,
    action: &mut Action,
) {
    const CELL_W: f32 = 124.0;
    const CELL_H: f32 = 140.0;
    const IMG: f32 = 96.0;
    const GAP: f32 = 8.0;

    let total = app.tab().entries.len();
    let cols = (((ui.available_width() + GAP) / (CELL_W + GAP)).floor() as usize).max(1);
    let rows = total.div_ceil(cols);

    // L'écart vertical entre les lignes DOIT valoir GAP : `show_rows` s'en
    // sert pour placer la fenêtre de troncature et la hauteur totale.
    ui.spacing_mut().item_spacing.y = GAP;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, CELL_H, rows, |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    let start = row * cols;
                    for index in start..(start + cols).min(total) {
                        grid_cell(app, ui, index, CELL_W, CELL_H, IMG, ctrl, shift, action);
                    }
                });
            }
        });
}

/// Une cellule de la grille : réserve exacte `cell_w × cell_h` (fond peint
/// au rect), vignette ou icône centrée dans la moitié haute, nom tronqué (ou
/// renommage inline) dessous. Positions déterministes : la hauteur de rangée
/// est toujours exactement `cell_h`, indispensable à `show_rows` (sinon les
/// rangées se décalent et se chevauchent au scroll).
#[allow(clippy::too_many_arguments)]
fn grid_cell(
    app: &mut ExplorerApp,
    ui: &mut egui::Ui,
    index: usize,
    cell_w: f32,
    cell_h: f32,
    img_px: f32,
    ctrl: bool,
    shift: bool,
    action: &mut Action,
) {
    if index >= app.tab().entries.len() {
        return;
    }
    let entry = app.tab().entries[index].clone();
    let selected = app.tab().selected.contains(&entry.path);
    let renaming = app.tab().renaming.as_ref() == Some(&entry.path);

    // Réserve EXACTE : position et taille de la cellule sans ambiguïté.
    let (rect, _) = ui.allocate_exact_size(egui::vec2(cell_w, cell_h), egui::Sense::hover());
    let bg = if selected {
        palette::SELECTED
    } else if ui.rect_contains_pointer(rect) {
        palette::HOVER_NEUTRAL
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6.0, bg);

    let inner = rect.shrink2(egui::vec2(4.0, 6.0));

    // --- Zone image (carrée, centrée dans la moitié haute) ---
    let img_rect = egui::Rect::from_min_size(
        egui::pos2(inner.center().x - img_px / 2.0, inner.top()),
        egui::vec2(img_px, img_px),
    );
    let mut img_ui = ui.new_child(egui::UiBuilder::new().max_rect(img_rect).layout(
        egui::Layout::centered_and_justified(egui::Direction::TopDown),
    ));
    let is_img = !entry.is_dir && crate::icons::is_previewable_file(&entry.extension);
    if is_img && let Some(tex) = app.icons.thumbnail(ui.ctx(), &entry.path, img_px as u32) {
        img_ui.add(egui::Image::new(egui::load::SizedTexture::new(
            tex.id(),
            egui::vec2(img_px, img_px),
        )));
    } else {
        let px = if entry.is_dir { 64.0 } else { 56.0 };
        add_tile_icon(app, &mut img_ui, &entry, px);
    }

    // --- Nom (tronqué) ou renommage inline, sous l'image ---
    let name_rect = egui::Rect::from_min_size(
        egui::pos2(inner.left(), img_rect.bottom() + 8.0),
        egui::vec2(inner.width(), 18.0),
    );
    let mut name_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(name_rect)
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    if renaming {
        let te = name_ui.add(
            egui::TextEdit::singleline(&mut app.tab_mut().rename_buffer)
                .desired_width(cell_w - 20.0),
        );
        if te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let path = entry.path.clone();
            let name = app.tab().rename_buffer.clone();
            app.rename_entry(&path, &name);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.tab_mut().renaming = None;
        }
    } else {
        let text = ellipsize(
            ui,
            &entry.name,
            cell_w - 16.0,
            egui::FontId::proportional(12.0),
        );
        let color = if entry.is_hidden {
            palette::TEXT_DIM
        } else {
            palette::TEXT
        };
        name_ui.label(RichText::new(text).color(color));
    }

    tile_response(app, ui, rect, index, ctrl, shift, action);
}

/// Ajoute l'icône `px` d'une entrée dans `ui` : icône système réelle
/// (macOS) si disponible, sinon l'emoji de repli. Retourne `true` si une
/// texture a été affichée.
fn add_row_icon(app: &mut ExplorerApp, ui: &mut egui::Ui, entry: &fs::Entry, px: f32) -> bool {
    if let Some(tex) = app.icons.system(ui.ctx(), &entry.path, px as u32) {
        ui.add(egui::Image::new(egui::load::SizedTexture::new(
            tex.id(),
            egui::vec2(px, px),
        )));
        return true;
    }
    let emoji = if entry.is_dir {
        folder_icon()
    } else {
        file_icon(&entry.extension)
    };
    ui.add(egui::Label::new(RichText::new(emoji).size(px * 0.9)).selectable(false));
    false
}

/// Grande icône centrée d'une cellule « Miniatures » : texture système ou
/// emoji de repli.
fn add_tile_icon(app: &mut ExplorerApp, ui: &mut egui::Ui, entry: &fs::Entry, px: f32) {
    if let Some(tex) = app.icons.system(ui.ctx(), &entry.path, px as u32) {
        ui.add(egui::Image::new(egui::load::SizedTexture::new(
            tex.id(),
            egui::vec2(px, px),
        )));
        return;
    }
    let emoji = if entry.is_dir {
        folder_icon()
    } else {
        file_icon(&entry.extension)
    };
    ui.add(egui::Label::new(RichText::new(emoji).size(px * 0.75)).selectable(false));
}

/// Message « dossier vide » centré (vues Liste et Miniatures).
fn show_empty_folder(ui: &mut egui::Ui) {
    ui.add_space(20.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("Ce dossier est vide.").color(palette::TEXT_DIM));
    });
}

/// Interaction commune des vues Liste / Miniatures : flash animé, survol,
/// sélection (clic / Ctrl / Maj), double-clic pour ouvrir et menu
/// contextuel — la fin de `render_row`, réutilisée pour les vues en tuiles.
fn tile_response(
    app: &mut ExplorerApp,
    ui: &egui::Ui,
    rect: egui::Rect,
    index: usize,
    ctrl: bool,
    shift: bool,
    action: &mut Action,
) -> egui::Response {
    if index >= app.tab().entries.len() {
        return ui.interact(rect, ui.id().with(("tile", index)), egui::Sense::click());
    }
    let entry = app.tab().entries[index].clone();

    // Flash animé vert (élément concerné par une action récente).
    if let Some(flash) = row_flash(app, &entry.path) {
        ui.painter().rect_filled(
            rect,
            6.0,
            Color32::from_rgba_unmultiplied(46, 165, 90, (130.0 * flash) as u8),
        );
        ui.ctx().request_repaint();
    }

    let resp = ui.interact(rect, ui.id().with(("tile", index)), egui::Sense::click());
    if resp.clicked() {
        app.handle_click(index, ctrl, shift);
    }
    // Double-clic : ouvrir (action différée, comme dans `render_row`).
    if resp.double_clicked() {
        *action = Action::Open(entry.path.clone());
    }
    if resp.hovered() && entry.is_dir {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.context_menu(|ui| {
        if index < app.tab().entries.len() {
            let item = app.tab().entries[index].clone();
            context_menu(app, ui, &item, index, action);
        }
    });
    resp
}

/// Intensité du flash (0.0 → 1.0) sur l'élément surligné après une action.
fn row_flash(app: &ExplorerApp, path: &PathBuf) -> Option<f32> {
    let (hl_path, at) = app.tab().highlight.as_ref()?;
    if hl_path != path {
        return None;
    }
    let age = at.elapsed().as_secs_f32();
    if age >= HIGHLIGHT_SECS {
        return None;
    }
    let t = age / HIGHLIGHT_SECS; // 0 → 1
    // Fondu décroissant avec une double pulsation : le vert « bat » puis s'efface.
    let pulse = 0.65 + 0.35 * (t * std::f32::consts::PI * 4.0).sin().abs();
    Some((1.0 - t) * pulse)
}

/// Largeur fixe des menus contextuels (style Windows 11).
const MENU_WIDTH: f32 = 280.0;

/// Item de menu contextuel style Windows 11 : icône, libellé, raccourci
/// clavier aligné à droite, surbrillance arrondie au survol. L'action par
/// défaut est rendue en gras. Retourne `true` si cliqué (et activé).
fn menu_item(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    shortcut: &str,
    strong: bool,
    enabled: bool,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(MENU_WIDTH, 30.0), egui::Sense::click());
    let text_color = if enabled {
        palette::TEXT
    } else {
        palette::TEXT_DIM
    };

    // Surbrillance du survol (arrondie, gris neutre comme sous Windows 11).
    if enabled && (resp.hovered() || resp.is_pointer_button_down_on()) {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                rect.min + egui::vec2(4.0, 1.0),
                egui::vec2(MENU_WIDTH - 8.0, rect.height() - 2.0),
            ),
            6.0,
            palette::MENU_HOVER,
        );
    }

    let mid = rect.center().y;
    // Colonne d'icône.
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, mid),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(15.0),
        text_color,
    );
    // Libellé (l'action par défaut est doublée à 0,5 px : effet gras garanti
    // même sans variante bold de la police système).
    let label_pos = egui::pos2(rect.left() + 42.0, mid);
    let label_font = egui::FontId::proportional(14.0);
    ui.painter().text(
        label_pos,
        egui::Align2::LEFT_CENTER,
        label,
        label_font.clone(),
        text_color,
    );
    if strong {
        ui.painter().text(
            label_pos + egui::vec2(0.5, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            label_font,
            text_color,
        );
    }
    // Raccourci clavier, aligné à droite en gris discret.
    if !shortcut.is_empty() {
        ui.painter().text(
            egui::pos2(rect.right() - 12.0, mid),
            egui::Align2::RIGHT_CENTER,
            shortcut,
            egui::FontId::proportional(12.0),
            palette::TEXT_DIM,
        );
    }

    enabled && resp.clicked()
}

/// Séparateur horizontal dans un menu contextuel (marges latérales).
fn menu_separator(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(MENU_WIDTH, 10.0), egui::Sense::hover());
    let y = rect.center().y;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 12.0, y),
            egui::pos2(rect.right() - 12.0, y),
        ],
        egui::Stroke::new(1.0, palette::BORDER),
    );
}

/// Sous-menu à flèche (▸), comme les entrées « Nouveau », « Trier par »…
/// de l'Explorateur Windows 11.
fn menu_submenu(ui: &mut egui::Ui, icon: &str, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.menu_button(
        RichText::new(format!("{icon}   {label}"))
            .size(14.0)
            .color(palette::TEXT),
        |sub_ui| {
            sub_ui.set_width(MENU_WIDTH);
            add(sub_ui);
        },
    );
}

/// Menu contextuel (clic droit) sur un élément, style Windows 11 :
/// action par défaut en gras, icônes, séparateurs et raccourcis à droite.
fn context_menu(
    app: &mut ExplorerApp,
    ui: &mut egui::Ui,
    entry: &fs::Entry,
    index: usize,
    action: &mut Action,
) {
    // Clic droit sur un élément non sélectionné : on le sélectionne d'abord
    // (comme dans l'Explorateur Windows) pour que les actions lui portent.
    if !app.tab().selected.contains(&entry.path) {
        app.handle_click(index, false, false);
    }

    let icon = if entry.is_dir {
        folder_icon()
    } else {
        file_icon(&entry.extension)
    };
    let n = app.tab().selected.len();
    let clipboard_full = app.clipboard.is_some();

    // Action par défaut, en gras tout en haut (« Ouvrir »).
    if menu_item(ui, icon, "Ouvrir", "Entrée", true, true) {
        let path = entry.path.clone();
        app.open_entry(ui.ctx(), &path);
        ui.close();
    }
    menu_separator(ui);

    if menu_item(ui, "✂️", "Couper", "Ctrl+X", false, n > 0) {
        *action = Action::Cut;
        ui.close();
    }
    if menu_item(ui, "📋", "Copier", "Ctrl+C", false, n > 0) {
        *action = Action::Copy;
        ui.close();
    }
    if entry.is_dir && menu_item(ui, "📥", "Coller", "Ctrl+V", false, clipboard_full) {
        let dest = entry.path.clone();
        *action = Action::PasteIn(dest);
        ui.close();
    }
    menu_separator(ui);

    if menu_item(ui, "✏️", "Renommer", "F2", false, true) {
        // Renomme l'élément cliqué (renommage inline dans la ligne).
        app.tab_mut().rename_buffer = entry.name.clone();
        app.tab_mut().renaming = Some(entry.path.clone());
        ui.close();
    }
    if menu_item(ui, "🗑️", "Supprimer", "Suppr", false, n > 0) {
        app.dialog = Dialog::ConfirmDelete {
            paths: app.tab().selected.iter().cloned().collect(),
        };
        ui.close();
    }
    menu_separator(ui);

    if menu_item(ui, "ℹ️", "Propriétés", "", false, true) {
        app.dialog = Dialog::Properties {
            path: entry.path.clone(),
        };
        ui.close();
    }
}

/// Menu contextuel sur le vide de la liste (clic droit hors des lignes),
/// style Windows 11 : Coller, Actualiser, sous-menus Nouveau ▸,
/// Trier par ▸, Afficher ▸ et Propriétés du dossier.
fn background_context_menu(app: &mut ExplorerApp, ui: &mut egui::Ui, action: &mut Action) {
    if menu_item(ui, "📥", "Coller", "Ctrl+V", false, app.clipboard.is_some()) {
        *action = Action::Paste;
        ui.close();
    }
    menu_separator(ui);
    if menu_item(ui, "🔄", "Actualiser", "F5", false, true) {
        *action = Action::Refresh;
        ui.close();
    }
    menu_separator(ui);

    // Nouveau ▸ : dossier / document texte.
    menu_submenu(ui, "➕", "Nouveau", |sub_ui| {
        if menu_item(sub_ui, "📁", "Dossier", "", false, true) {
            *action = Action::NewFolder;
            sub_ui.close();
        }
        if menu_item(sub_ui, "📄", "Document texte", "", false, true) {
            *action = Action::NewFile;
            sub_ui.close();
        }
    });

    // Trier par ▸ (la coche suit la colonne courante).
    menu_submenu(ui, "🔀", "Trier par", |sub_ui| {
        for (col, label) in [
            (SortColumn::Name, "Nom"),
            (SortColumn::Type, "Type"),
            (SortColumn::Size, "Taille"),
            (SortColumn::Modified, "Modifié le"),
        ] {
            let current = app.tab().sort_column == col;
            if menu_item(
                sub_ui,
                if current { "✓" } else { "" },
                label,
                "",
                false,
                true,
            ) {
                app.toggle_sort(col);
                *action = Action::Refresh;
                sub_ui.close();
            }
        }
    });

    // Afficher ▸ : fichiers cachés.
    menu_submenu(ui, "👁️", "Afficher", |sub_ui| {
        let on = app.tab().show_hidden;
        if menu_item(
            sub_ui,
            if on { "✓" } else { "" },
            "Fichiers cachés",
            "",
            false,
            true,
        ) {
            *action = Action::ToggleHidden;
            sub_ui.close();
        }
    });

    menu_separator(ui);
    if menu_item(ui, "ℹ️", "Propriétés du dossier", "", false, true) {
        let path = app.tab().cwd.clone();
        app.dialog = Dialog::Properties { path };
        ui.close();
    }
}

/// Durée du flash de surlignage après une action, en secondes.
const HIGHLIGHT_SECS: f32 = 2.0;

/// Affiche la notification (toast) animée en bas à droite :
/// glissement depuis la droite, fondu d'entrée/sortie, icône pulsée.
fn render_toast(app: &mut ExplorerApp, ctx: &egui::Context) {
    // On retire le toast de l'état pendant le rendu (évite les conflits
    // d'emprunt) puis on le remet en place s'il est encore valide.
    let Some(toast) = app.toast.take() else {
        return;
    };
    let age = toast.created_at.elapsed().as_secs_f32();
    if age >= toast.lifetime {
        return; // expiré : il reste retiré de l'état.
    }
    // Les animations exigent un rafraîchissement continu des frames.
    ctx.request_repaint();

    // Fondu : entrée rapide (0,15 s), sortie sur les 0,45 s finales.
    let fade_in = (age / 0.15).clamp(0.0, 1.0);
    let fade_out = ((toast.lifetime - age) / 0.45).clamp(0.0, 1.0);
    let alpha = fade_in * fade_out;
    // Glissement : décalage de 48 px qui se résorbe (accélération douce).
    let slide = 1.0 - fade_in;
    let offset_x = slide * slide * 48.0;
    // Pulsation de l'icône : petit rebond sinusoïdal amorti.
    let bounce = (age * 9.0).sin().abs() * (-age * 1.8).exp();
    let icon_size = 20.0 + 7.0 * bounce;
    let text_alpha = (255.0 * alpha) as u8;

    let icon = toast.icon.clone();
    let message = toast.message.clone();

    egui::Area::new(egui::Id::new("toast_area"))
        .order(egui::Order::Foreground)
        .anchor(
            egui::Align2::RIGHT_BOTTOM,
            egui::vec2(-18.0 + offset_x, -46.0),
        )
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(
                    32,
                    32,
                    36,
                    (242.0 * alpha) as u8,
                ))
                .corner_radius(12.0)
                .inner_margin(egui::Margin::symmetric(14, 10))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 12,
                    spread: 0,
                    color: Color32::from_black_alpha(70),
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(icon.as_str())
                                .size(icon_size)
                                .color(Color32::from_white_alpha(text_alpha)),
                        );
                        ui.label(
                            RichText::new(message.as_str())
                                .size(14.0)
                                .color(Color32::from_white_alpha(text_alpha)),
                        );
                    });
                });
        });

    app.toast = Some(toast);
}

/// Affiche la boîte de dialogue modale active (le cas échéant).
/// Boîte de dialogue modale : déverrouillage du magasin d'identifiants avec le mot de passe maître.
fn show_unlock_dialog(app: &mut ExplorerApp, ctx: &egui::Context) {
    let mut open = true;
    let mut close = false;
    egui::Window::new("Déverrouiller le magasin d'identifiants")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(
                "Entrez le mot de passe maître utilisé pour chiffrer les identifiants des serveurs distants :",
            );
            ui.add_space(6.0);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.unlock_dialog.password)
                    .password(true)
                    .desired_width(300.0),
            );
            let unlock = (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                || ui.button("Déverrouiller").clicked();
            if unlock {
                let password = app.unlock_dialog.password.clone();
                match app.unlock_credentials(password) {
                    Ok(()) => {
                        app.unlock_dialog.password.clear();
                        close = true;
                    }
                    Err(e) => {
                        app.remote_error = Some(e);
                    }
                }
            }
            if let Some(e) = &app.remote_error {
                ui.add_space(6.0);
                ui.colored_label(egui::Color32::from_rgb(200, 60, 60), e);
            }
        });
    if !open || close {
        app.unlock_dialog.open = false;
        if close {
            app.remote_error = None;
        }
    }
}

/// Boîte de dialogue modale : connexion à un serveur distant (SSH/SFTP ou FTP).
fn show_remote_connect_dialog(app: &mut ExplorerApp, ctx: &egui::Context) {
    if !app.remote_dialog.open {
        return;
    }
    let mut close = false;
    egui::Window::new("Connexion à un serveur distant")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Protocole :");
            egui::ComboBox::from_id_salt("remote_protocol")
                .selected_text(format!("{}", app.remote_dialog.protocol))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut app.remote_dialog.protocol,
                        remote::RemoteProtocol::Ssh,
                        "SSH / SFTP",
                    );
                    ui.selectable_value(
                        &mut app.remote_dialog.protocol,
                        remote::RemoteProtocol::Ftp,
                        "FTP",
                    );
                });
            ui.add_space(6.0);
            ui.label("Hôte :");
            ui.text_edit_singleline(&mut app.remote_dialog.host);
            ui.add_space(6.0);
            ui.label("Port :");
            ui.text_edit_singleline(&mut app.remote_dialog.port);
            ui.add_space(6.0);
            ui.label("Utilisateur :");
            ui.text_edit_singleline(&mut app.remote_dialog.user);
            ui.add_space(6.0);
            ui.label("Mot de passe :");
            ui.add(egui::TextEdit::singleline(&mut app.remote_dialog.password).password(true));
            ui.add_space(6.0);
            ui.checkbox(
                &mut app.remote_dialog.save,
                "Enregistrer le serveur (mot de passe chiffré)",
            );
            if let Some(e) = &app.remote_error {
                ui.add_space(6.0);
                ui.colored_label(egui::Color32::from_rgb(200, 60, 60), e);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if primary_button(ui, "Se connecter") {
                    let save = app.remote_dialog.save;
                    let port: u16 = app.remote_dialog.port.trim().parse().unwrap_or(22);
                    let cfg = remote::RemoteConfig {
                        id: format!(
                            "{}@{}:{}",
                            app.remote_dialog.user, app.remote_dialog.host, port
                        ),
                        name: app.remote_dialog.host.clone(),
                        protocol: app.remote_dialog.protocol,
                        host: app.remote_dialog.host.clone(),
                        port,
                        user: app.remote_dialog.user.clone(),
                        password: Some(app.remote_dialog.password.clone()),
                    };
                    app.connect_remote(cfg, save, ctx);
                    close = true;
                }
                if ui
                    .add_sized([88.0, 32.0], egui::Button::new("Annuler"))
                    .clicked()
                {
                    app.remote_dialog.open = false;
                    close = true;
                }
            });
        });
    if close {
        app.remote_dialog.open = false;
    }
}

fn show_dialog(app: &mut ExplorerApp, ctx: &egui::Context) {
    // On extrait le dialogue pour éviter d'emprunter `app` dans la fenêtre.
    let dialog = std::mem::replace(&mut app.dialog, Dialog::None);

    match dialog {
        Dialog::None => {}
        Dialog::NewFolder { mut name } => {
            let mut open = true;
            let mut close = false;
            egui::Window::new("Nouveau dossier")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Nom du dossier :");
                    let resp = ui.add(egui::TextEdit::singleline(&mut name).desired_width(260.0));
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        close = app.create_new_folder(&name);
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if primary_button(ui, "Créer") {
                            close = app.create_new_folder(&name);
                        }
                        if ui
                            .add_sized([88.0, 32.0], egui::Button::new("Annuler"))
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
            // Le dialogue a été extrait de l'état ci-dessus : on le remet en place
            // tant qu'il n'a pas été fermé, sinon il ne survivrait qu'un seul frame.
            if open && !close {
                app.dialog = Dialog::NewFolder { name };
            }
        }
        Dialog::NewFile { mut name } => {
            let mut open = true;
            let mut close = false;
            egui::Window::new("Nouveau fichier")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Nom du fichier :");
                    let resp = ui.add(egui::TextEdit::singleline(&mut name).desired_width(260.0));
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        close = app.create_new_file(&name);
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if primary_button(ui, "Créer") {
                            close = app.create_new_file(&name);
                        }
                        if ui
                            .add_sized([88.0, 32.0], egui::Button::new("Annuler"))
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
            if open && !close {
                app.dialog = Dialog::NewFile { name };
            }
        }

        Dialog::Rename { path, mut name } => {
            let mut open = true;
            let mut close = false;
            egui::Window::new("Renommer")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Nouveau nom :");
                    let resp = ui.add(egui::TextEdit::singleline(&mut name).desired_width(260.0));
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        close = app.rename_entry(&path, &name);
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if primary_button(ui, "Renommer") {
                            close = app.rename_entry(&path, &name);
                        }
                        if ui
                            .add_sized([88.0, 32.0], egui::Button::new("Annuler"))
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
            if open && !close {
                app.dialog = Dialog::Rename { path, name };
            }
        }
        Dialog::ConfirmDelete { paths } => {
            let count = paths.len();
            let mut open = true;
            let mut close = false;
            egui::Window::new("Confirmer la suppression")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(format!("Déplacer {count} élément(s) vers la corbeille ?"));
                    if count <= 5 {
                        for p in &paths {
                            let n = p
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| p.to_string_lossy().to_string());
                            ui.label(RichText::new(format!("  • {n}")).color(palette::TEXT_DIM));
                        }
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if primary_button(ui, "Supprimer") {
                            // On supprime exactement les chemins listés dans le dialogue.
                            app.delete_paths(&paths);
                            close = true;
                        }
                        if ui
                            .add_sized([88.0, 32.0], egui::Button::new("Annuler"))
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
            if open && !close {
                app.dialog = Dialog::ConfirmDelete { paths };
            }
        }

        Dialog::Properties { path } => {
            let mut open = true;
            let mut close = false;
            egui::Window::new("Propriétés")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    if let Some(entry) = fs::Entry::from_path(&path) {
                        egui::Grid::new("props_grid")
                            .num_columns(2)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(RichText::new("Nom :").strong());
                                ui.label(&entry.name);
                                ui.end_row();

                                ui.label(RichText::new("Type :").strong());
                                ui.label(entry.type_label());
                                ui.end_row();

                                ui.label(RichText::new("Emplacement :").strong());
                                let parent = path
                                    .parent()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                ui.label(parent);
                                ui.end_row();

                                if !entry.is_dir {
                                    ui.label(RichText::new("Taille :").strong());
                                    ui.label(format!(
                                        "{} ({} octets)",
                                        fs::format_size(entry.size),
                                        entry.size
                                    ));
                                    ui.end_row();
                                }

                                ui.label(RichText::new("Modifié le :").strong());
                                let dt = entry
                                    .modified
                                    .map(|d| format_datetime(&d))
                                    .unwrap_or_else(|| "—".to_string());
                                ui.label(dt);
                                ui.end_row();

                                ui.label(RichText::new("Caché :").strong());
                                ui.label(if entry.is_hidden { "Oui" } else { "Non" });
                                ui.end_row();
                            });
                    }
                    ui.add_space(6.0);
                    if ui
                        .add_sized([88.0, 32.0], egui::Button::new("Fermer"))
                        .clicked()
                    {
                        close = true;
                    }
                });
            if open && !close {
                app.dialog = Dialog::Properties { path };
            }
        }

        Dialog::SystemProperties => {
            let mut open = true;
            let mut close = false;

            let sys_info = {
                let mut sys = System::new_all();
                sys.refresh_all();
                let long_os = System::long_os_version()
                    .unwrap_or_else(|| System::os_version().unwrap_or_default());
                // Sépare nom, version et « code name » (ex: "macOS 15.1.1 Sequoia").
                let parts: Vec<&str> = long_os.split_whitespace().collect();
                let os_name = parts.first().copied().unwrap_or("").to_string();
                let os_version = parts.get(1).copied().unwrap_or("").to_string();
                let os_code_name = parts.get(2..).map(|r| r.join(" ")).unwrap_or_default();
                let cpus = sys.cpus();
                let cpu_brand = cpus
                    .first()
                    .map(|c| c.brand().to_string())
                    .unwrap_or_else(|| "—".to_string());
                let cpu_cores = sys
                    .physical_core_count()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "—".to_string());
                let cpu_freq_mhz = cpus.first().map(|c| c.frequency()).unwrap_or(0);
                let ram_total = sys.total_memory();
                let ram_available = sys.available_memory();
                let ram_used = sys.used_memory();
                let disks: Vec<(String, u64, u64, String)> = Disks::new_with_refreshed_list()
                    .list()
                    .iter()
                    .map(|d| {
                        let name = d.mount_point().to_string_lossy().to_string();
                        let type_label = match d.kind() {
                            DiskKind::HDD => "HDD".to_string(),
                            DiskKind::SSD => "SSD".to_string(),
                            DiskKind::Unknown(_) => "Inconnu".to_string(),
                        };
                        (name, d.total_space(), d.available_space(), type_label)
                    })
                    .collect();
                SystemInfo {
                    os_name,
                    os_version,
                    os_code_name,
                    cpu_brand,
                    cpu_cores,
                    cpu_freq_mhz,
                    ram_total,
                    ram_available,
                    ram_used,
                    disks,
                }
            };

            egui::Window::new("Propriétés de l'ordinateur")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_width(520.0);
                    ui.horizontal(|ui| {
                        let os_emoji = match std::env::consts::OS {
                            "macos" => "🍎",
                            "linux" => "🐧",
                            "windows" => "🪟",
                            _ => "💻",
                        };
                        ui.label(RichText::new(os_emoji).size(32.0));
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Propriétés de l'ordinateur")
                                    .strong()
                                    .size(20.0),
                            );
                            ui.label(RichText::new(&sys_info.os_name).color(palette::TEXT_DIM));
                        });
                    });
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(12.0);

                    egui::Grid::new("system_props_grid")
                        .num_columns(2)
                        .spacing([16.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("Système d'exploitation :").strong());
                            let mut os_label = sys_info.os_name.clone();
                            if !sys_info.os_version.is_empty() {
                                os_label.push_str(&format!(" {}", sys_info.os_version));
                            }
                            if !sys_info.os_code_name.is_empty() {
                                os_label.push_str(&format!(" — {}", sys_info.os_code_name));
                            }
                            ui.label(os_label);
                            ui.end_row();
                            ui.label(RichText::new("Processeur :").strong());
                            ui.label(&sys_info.cpu_brand);
                            ui.end_row();
                            ui.label(RichText::new("Cœurs physiques :").strong());
                            ui.label(&sys_info.cpu_cores);
                            ui.end_row();
                            if sys_info.cpu_freq_mhz > 0 {
                                ui.label(RichText::new("Fréquence :").strong());
                                ui.label(format!("{} MHz", sys_info.cpu_freq_mhz));
                                ui.end_row();
                            }
                            ui.label(RichText::new("Mémoire vive (RAM) :").strong());
                            ui.label(format!(
                                "{} / {} ({} disponible)",
                                fs::format_size(sys_info.ram_used),
                                fs::format_size(sys_info.ram_total),
                                fs::format_size(sys_info.ram_available)
                            ));
                            ui.end_row();
                            if sys_info.disks.is_empty() {
                                ui.label(RichText::new("Disques :").strong());
                                ui.label("Aucun disque détecté");
                                ui.end_row();
                            } else {
                                ui.label(RichText::new("Disques :").strong());
                                let storage_text: Vec<String> = sys_info
                                    .disks
                                    .iter()
                                    .map(|(name, total, available, type_label)| {
                                        format!(
                                            "{} ({}) — {} / {}",
                                            name,
                                            type_label,
                                            fs::format_size(*available),
                                            fs::format_size(*total)
                                        )
                                    })
                                    .collect();
                                ui.label(storage_text.join("\n"));
                                ui.end_row();
                            }
                        });

                    ui.add_space(16.0);
                    if ui
                        .add_sized([88.0, 32.0], egui::Button::new("Fermer"))
                        .clicked()
                    {
                        close = true;
                    }
                });
            if open && !close {
                app.dialog = Dialog::SystemProperties;
            }
        }

        Dialog::FileSizeMap { mut min_size } => {
            let mut open = true;
            let mut close = false;

            // Scanner depuis la racine du système de fichiers en premier,
            // puis le home, puis le dossier courant comme dernier recours.
            let cwd = app.tab().cwd.clone();
            let show_hidden = app.tab().show_hidden;

            // 1. Racine du système de fichiers
            let root = fs::get_roots()
                .first()
                .cloned()
                .unwrap_or_else(|| PathBuf::from("/"));
            let mut entries = fs::list_dir(&root, show_hidden).unwrap_or_default();
            let mut scan_path = root.clone();

            // 2. Home (fallback si la racine est illisible, ex. SIP sur macOS)
            if entries.is_empty() {
                if let Some(home) = fs::home_dir() {
                    entries = fs::list_dir(&home, show_hidden).unwrap_or_default();
                    scan_path = home;
                }
            }

            // 3. Dossier courant (dernier recours)
            if entries.is_empty() {
                entries = fs::list_dir(&cwd, show_hidden).unwrap_or_default();
                scan_path = cwd.clone();
            }

            // 4. Chemins système courants (sur macOS, /Users est souvent lisible même si / ne l'est pas)
            if entries.is_empty() {
                for candidate in ["/Users", "/Volumes", "/tmp"] {
                    let p = PathBuf::from(candidate);
                    if p.exists() {
                        let e = fs::list_dir(&p, show_hidden).unwrap_or_default();
                        if !e.is_empty() {
                            entries = e;
                            scan_path = p;
                            break;
                        }
                    }
                }
            }

            let mut files: Vec<&fs::Entry> = entries
                .iter()
                .filter(|e| !e.is_dir && e.size >= min_size)
                .collect();
            files.sort_by_key(|a| std::cmp::Reverse(a.size));

            let dir_count = entries.iter().filter(|e| e.is_dir).count();
            let file_count = entries.len() - dir_count;

            // ── Mode démo : si aucun fichier n'a été trouvé, on génère des
            // entrées de test pour que le rendu des carrés soit visible.
            let demo_mode = files.is_empty() && entries.is_empty();
            let mut demo_entries: Vec<fs::Entry> = Vec::new();
            if demo_mode {
                demo_entries = vec![
                    fs::Entry {
                        path: PathBuf::from("demo/petite_photo.jpg"),
                        name: "petite_photo.jpg".into(),
                        is_dir: false,
                        is_hidden: false,
                        size: 50_000,
                        modified: None,
                        extension: "jpg".into(),
                        is_symlink: false,
                    },
                    fs::Entry {
                        path: PathBuf::from("demo/document.pdf"),
                        name: "document.pdf".into(),
                        is_dir: false,
                        is_hidden: false,
                        size: 2_000_000,
                        modified: None,
                        extension: "pdf".into(),
                        is_symlink: false,
                    },
                    fs::Entry {
                        path: PathBuf::from("demo/video.mp4"),
                        name: "video.mp4".into(),
                        is_dir: false,
                        is_hidden: false,
                        size: 150_000_000,
                        modified: None,
                        extension: "mp4".into(),
                        is_symlink: false,
                    },
                    fs::Entry {
                        path: PathBuf::from("demo/fichier_texte.txt"),
                        name: "fichier_texte.txt".into(),
                        is_dir: false,
                        is_hidden: false,
                        size: 1_200,
                        modified: None,
                        extension: "txt".into(),
                        is_symlink: false,
                    },
                    fs::Entry {
                        path: PathBuf::from("demo/archive.zip"),
                        name: "archive.zip".into(),
                        is_dir: false,
                        is_hidden: false,
                        size: 45_000_000,
                        modified: None,
                        extension: "zip".into(),
                        is_symlink: false,
                    },
                    fs::Entry {
                        path: PathBuf::from("demo/iso_image.iso"),
                        name: "iso_image.iso".into(),
                        is_dir: false,
                        is_hidden: false,
                        size: 700_000_000,
                        modified: None,
                        extension: "iso".into(),
                        is_symlink: false,
                    },
                ];
                scan_path = PathBuf::from("(démo)");
            }

            egui::Window::new("Carte des tailles de fichiers")
                .collapsible(false)
                .resizable(true)
                .default_size([700.0, 500.0])
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Taille minimum :");
                        // Champ numérique (DragValue) pour saisir la taille en octets
                        let mut min_size_f = min_size as f64;
                        let resp = ui.add(
                            egui::DragValue::new(&mut min_size_f)
                                .speed(1.0)
                                .suffix(" o"),
                        );
                        if resp.changed() && !demo_mode {
                            min_size = min_size_f.max(0.0) as u64;
                            files = entries
                                .iter()
                                .filter(|e| !e.is_dir && e.size >= min_size)
                                .collect();
                            files.sort_by_key(|a| std::cmp::Reverse(a.size));
                        }
                        ui.label(format!("({})", fs::format_size(min_size)));
                        ui.separator();
                        if ui.button("Tout afficher (0 o)").clicked() && !demo_mode {
                            min_size = 0;
                            files = entries
                                .iter()
                                .filter(|e| !e.is_dir && e.size >= min_size)
                                .collect();
                            files.sort_by_key(|a| std::cmp::Reverse(a.size));
                        }
                        ui.separator();
                        let display_count = if demo_mode { demo_entries.len() } else { files.len() };
                        ui.label(format!("{} fichier(s)", display_count));
                        ui.separator();
                        ui.label(format!("Dossier : {}", scan_path.display()));
                        ui.separator();
                        ui.label(format!(
                            "{} entrée(s) ({} fichier(s), {} dossier(s)){}",
                            entries.len(),
                            file_count,
                            dir_count,
                            if demo_mode { " [démo]" } else { "" }
                        ));
                    });

                    // Déterminer quels fichiers afficher (réels ou démo)
                    let render_files: Vec<&fs::Entry> = if demo_mode {
                        demo_entries.iter().collect()
                    } else {
                        files.clone()
                    };

                    // Afficher le message démo si applicable
                    if demo_mode {
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("🎮 Mode démo — aucun fichier réel n'a été trouvé.")
                                .color(Color32::from_rgb(100, 150, 255))
                                .italics(),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "Les carrés ci-dessous représentent des fichiers de test (petit=vert, gros=rouge).",
                            )
                            .size(10.0)
                            .color(palette::TEXT_DIM),
                        );
                    } else if entries.is_empty() {
                        ui.add_space(40.0);
                        ui.label(
                            RichText::new("⚠ Impossible de lire ce dossier.")
                                .color(Color32::from_rgb(255, 80, 80))
                                .strong(),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(format!("Chemin essayé : {}", scan_path.display()))
                                .size(11.0)
                                .color(palette::TEXT_DIM),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(
                                "Aucun dossier lisible n'a été trouvé. Essayez de lancer l'application depuis un dossier contenant des fichiers.",
                            )
                            .size(10.0)
                            .color(palette::TEXT_DIM)
                            .italics(),
                        );
                        ui.add_space(40.0);
                    } else if files.is_empty() {
                        ui.add_space(40.0);
                        ui.label(
                            RichText::new("ℹ Aucun fichier à afficher.")
                                .color(palette::TEXT_DIM)
                                .italics(),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(format!(
                                "Le dossier contient {} dossier(s) mais aucun fichier (ou tous les fichiers sont exclus par le filtre de taille).",
                                dir_count
                            ))
                            .size(11.0)
                            .color(palette::TEXT_DIM),
                        );
                        ui.add_space(10.0);
                        if ui.button("Réduire le filtre de taille à 0").clicked() {
                            min_size = 0;
                            files = entries
                                .iter()
                                .filter(|e| !e.is_dir && e.size >= min_size)
                                .collect();
                            files.sort_by_key(|a| std::cmp::Reverse(a.size));
                        }
                        ui.add_space(40.0);
                    }

                    // Dessiner les carrés (mode démo ou fichiers réels)
                    if !render_files.is_empty() {
                        ui.add_space(12.0);

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let available_width = ui.available_width();
                            let mut x = 0.0;
                            let mut y = 0.0;
                            let mut row_height = 0.0;
                            const PADDING: f32 = 6.0;

                            let max_size = render_files
                                .first()
                                .map(|f| f.size)
                                .unwrap_or(1) as f64;
                            let max_size = if max_size <= 0.0 { 1.0 } else { max_size };
                            const MAX_SIDE: f32 = 140.0;
                            const MIN_SIDE: f32 = 30.0;

                            for entry in &render_files {
                                let ratio = (entry.size as f64 / max_size).sqrt() as f32;
                                let side = MIN_SIDE + ratio * (MAX_SIDE - MIN_SIDE);

                                if x + side > available_width && x > 0.0 {
                                    x = 0.0;
                                    y += row_height + PADDING;
                                    row_height = 0.0;
                                }

                                let rect = egui::Rect::from_min_max(
                                    egui::pos2(x, y),
                                    egui::pos2(x + side, y + side),
                                );

                                let color = file_color(entry, max_size);
                                ui.painter().rect_filled(rect, 4.0, color);
                                ui.painter().rect_stroke(
                                    rect,
                                    4.0,
                                    egui::Stroke::new(1.0, Color32::WHITE),
                                    egui::StrokeKind::Outside,
                                );

                                let text_color = if is_color_dark(color) {
                                    Color32::WHITE
                                } else {
                                    Color32::BLACK
                                };

                                let name = ellipsize(
                                    ui,
                                    &entry.name,
                                    (side - 8.0).max(0.0),
                                    egui::FontId::proportional(10.0),
                                );
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    name,
                                    egui::FontId::proportional(10.0),
                                    text_color,
                                );

                                let size_text = fs::format_size(entry.size);
                                ui.painter().text(
                                    egui::pos2(rect.center().x, rect.bottom() + 8.0),
                                    egui::Align2::CENTER_TOP,
                                    size_text,
                                    egui::FontId::proportional(10.0),
                                    palette::TEXT_DIM,
                                );

                                x += side + PADDING;
                                row_height = row_height.max(side + 24.0);
                            }

                            ui.add_space(y + row_height + 20.0);
                        });
                    }

                    ui.add_space(16.0);
                    if ui
                        .add_sized([88.0, 32.0], egui::Button::new("Fermer"))
                        .clicked()
                    {
                        close = true;
                    }
                });

            if open && !close {
                app.dialog = Dialog::FileSizeMap { min_size };
            }
        }

        Dialog::About => {
            let mut open = true;
            let mut close = false;

            egui::Window::new("À propos de SmartExplorer")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_width(430.0);

                    // Icône de l'OS hôte (repli emoji si l'icône système est indisponible).
                    let os_icon = app.icons.system(ui.ctx(), &PathBuf::from("/"), 48);
                    let os_name = match std::env::consts::OS {
                        "macos" => "macOS",
                        "linux" => "Linux",
                        "windows" => "Windows",
                        other => other,
                    };
                    let os_emoji = match std::env::consts::OS {
                        "macos" => "🍎",
                        "linux" => "🐧",
                        "windows" => "🪟",
                        _ => "💻",
                    };

                    // En-tête : icône OS + nom + version.
                    ui.horizontal(|ui| {
                        if let Some(tex) = os_icon {
                            ui.image(&tex);
                        } else {
                            ui.label(RichText::new(os_emoji).size(48.0));
                        }
                        ui.vertical(|ui| {
                            ui.label(RichText::new("SmartExplorer").strong().size(24.0));
                            ui.label(
                                RichText::new(format!(
                                    "v{} — {}",
                                    env!("CARGO_PKG_VERSION"),
                                    os_name
                                ))
                                .color(palette::TEXT_DIM),
                            );
                        });
                    });

                    ui.add_space(8.0);

                    ui.label(
                        RichText::new(
                            "Explorateur de fichiers multiplateforme en Rust (Windows, Linux, macOS), inspiré de l'Explorateur Windows 11.",
                        )
                        .color(palette::TEXT_DIM),
                    );

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Informations de publication.
                    ui.label(RichText::new("Développé par :").strong());
                    ui.label("Elhoussin Darrazi");
                    ui.add_space(4.0);
                    ui.label(RichText::new("Version :").strong());
                    ui.label(env!("CARGO_PKG_VERSION"));
                    ui.add_space(4.0);
                    ui.label(RichText::new("Date de publication :").strong());
                    ui.label("15 octobre 2026");

                    ui.add_space(16.0);

                    if ui
                        .add_sized([88.0, 32.0], egui::Button::new("Fermer"))
                        .clicked()
                    {
                        close = true;
                    }
                });
            if open && !close {
                app.dialog = Dialog::About;
            }
        }
    }
}

/// Barre d'état inférieure : filet discret en haut, message à gauche et
/// nombre d'éléments à droite (style Windows 11).
fn status_bar(app: &mut ExplorerApp, ui: &mut egui::Ui) {
    // Filet séparateur au sommet de la barre.
    let rect = ui.max_rect();
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.top() + 0.5),
            egui::pos2(rect.right(), rect.top() + 0.5),
        ],
        egui::Stroke::new(1.0, palette::BORDER),
    );
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            RichText::new(&app.tab().status)
                .size(13.0)
                .color(palette::TEXT_DIM),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            // Nombre d'éléments + sélection.
            let sel = app.tab().selected.len();
            if sel > 0 {
                ui.label(
                    RichText::new(format!("{sel} sélectionné(s)"))
                        .size(13.0)
                        .color(palette::TEXT_DIM),
                );
            }
        });
    });
    ui.add_space(5.0);
}

/// Charge, si possible, la police système (Segoe UI sous Windows, SF /
/// Helvetica sur macOS, DejaVu/Noto sous Linux) pour un rendu le plus proche
/// possible de Windows 11 ; repli silencieux sur les polices egui sinon.
fn load_system_font(ctx: &egui::Context) {
    fn candidates() -> Vec<std::path::PathBuf> {
        let mut v = Vec::new();
        if cfg!(windows) {
            if let Ok(windir) = std::env::var("WINDIR") {
                let fonts = std::path::Path::new(&windir).join("Fonts");
                v.push(fonts.join("segoeui.ttf")); // Segoe UI
            }
        } else if cfg!(target_os = "macos") {
            v.push("/System/Library/Fonts/SFNS.ttf".into()); // SF Pro
            v.push("/System/Library/Fonts/Helvetica.ttc".into());
            v.push("/System/Library/Fonts/Supplemental/Arial.ttf".into());
        } else {
            v.push("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".into());
            v.push("/usr/share/fonts/TTF/DejaVuSans.ttf".into());
            v.push("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf".into());
            v.push("/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf".into());
            v.push("/usr/share/fonts/noto/NotoSans-Regular.ttf".into());
        }
        v
    }
    fn is_font(bytes: &[u8]) -> bool {
        bytes.len() > 16
            && (bytes.starts_with(&[0x00, 0x01, 0x00, 0x00])
                || bytes.starts_with(b"true")
                || bytes.starts_with(b"OTTO")
                || bytes.starts_with(b"ttcf"))
    }
    let Some(bytes) = candidates()
        .into_iter()
        .find_map(|p| std::fs::read(p).ok())
        .filter(|b| is_font(b))
    else {
        return;
    };
    let mut defs = egui::FontDefinitions::default();
    defs.font_data.insert(
        "SystemUI".to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(bytes)),
    );
    if let Some(fam) = defs.families.get_mut(&egui::FontFamily::Proportional) {
        fam.insert(0, "SystemUI".to_owned());
    }
    ctx.set_fonts(defs);
}

/// Applique (une seule fois) le style Windows 11 : polices système, thème
/// clair, coins 8 px, accent bleu, ombres douces et scrollbars fines ;
/// ré-imposé si le système repasse en sombre.
fn ensure_style(ctx: &egui::Context) {
    fn win11_visuals() -> egui::Visuals {
        let mut visuals = egui::Visuals::light();

        // Fenêtres et popups : cartes blanches, bordure fine, coins 8 px.
        visuals.window_fill = Color32::WHITE;
        visuals.window_stroke = egui::Stroke::new(1.0, palette::BORDER);
        visuals.window_corner_radius = egui::CornerRadius::same(8);
        visuals.menu_corner_radius = egui::CornerRadius::same(8);
        visuals.window_shadow = egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(50),
        };
        visuals.popup_shadow = egui::epaint::Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(60),
        };
        // Fond des panneaux : Mica derrière les barres et le volet latéral.
        visuals.panel_fill = palette::TOOLBAR_BG;

        // Sélection : bleu clair Windows 11, texte sombre par-dessus.
        visuals.selection.bg_fill = palette::SELECTED;
        visuals.selection.stroke = egui::Stroke::new(1.0, palette::TEXT);

        // Contrôles (boutons, cases…) : neutres, bordures subtiles, coins 4 px.
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette::BORDER);
        visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(243, 243, 243);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(243, 243, 243);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Color32::from_black_alpha(30));
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(234, 234, 234);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(234, 234, 234);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Color32::from_black_alpha(60));
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette::TEXT);
        visuals.widgets.active.weak_bg_fill = Color32::from_rgb(224, 224, 224);
        visuals.widgets.active.bg_fill = Color32::from_rgb(224, 224, 224);
        visuals.widgets.open.weak_bg_fill = Color32::from_rgb(224, 224, 224);
        visuals.widgets.open.bg_fill = Color32::from_rgb(224, 224, 224);
        for w in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            w.corner_radius = egui::CornerRadius::same(4);
        }

        // Accents (liens, curseur de texte).
        visuals.hyperlink_color = palette::ACCENT;
        visuals.text_cursor.stroke = egui::Stroke::new(2.0, palette::ACCENT);
        visuals
    }

    static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        load_system_font(ctx);
        ctx.set_visuals(win11_visuals());
        ctx.global_style_mut(|style| {
            style.spacing.menu_margin = egui::Margin::symmetric(6, 6);
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
            style
                .text_styles
                .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
            style
                .text_styles
                .insert(egui::TextStyle::Heading, egui::FontId::proportional(18.0));
            // Barres de défilement flottantes et fines (style Windows 11).
            style.spacing.scroll.floating = true;
            style.spacing.scroll.bar_width = 12.0;
        });
    });
    // Si le thème système repasse en sombre, on ré-impose le thème clair
    // (une seule fois ensuite : pas de redemande de repaint permanente).
    if ctx.global_style().visuals.dark_mode {
        ctx.set_visuals(win11_visuals());
    }
}

/// Hauteur d'un onglet de la barre supérieure (style Windows 11).
const TAB_H: f32 = 34.0;

/// Barre d'onglets façon Windows 11 : onglet actif blanc à coins supérieurs
/// arrondis, bouton « + » et fermeture au survol / clic molette.
fn tab_bar(app: &mut ExplorerApp, ui: &mut egui::Ui) {
    let mut switch: Option<usize> = None;
    let mut close: Option<usize> = None;
    let mut new_tab = false;

    ui.horizontal(|ui| {
        ui.add_space(6.0);
        for i in 0..app.tabs.len() {
            let active = i == app.active_tab;
            let tab = &app.tabs[i];
            let name = tab.label.clone();
            let tab_icon = if tab.kind == TabKind::Remote {
                "🖧"
            } else {
                folder_icon()
            };

            // Largeur : texte + icône + bouton fermer, bornée [120 ; 240].
            let font = egui::FontId::proportional(14.0);
            let text_w = measure_text(ui, &name, font.clone());
            let tab_w = (text_w + 74.0).clamp(120.0, 240.0);
            let label = ellipsize(ui, &name, tab_w - 74.0, font.clone());

            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(tab_w, TAB_H), egui::Sense::click());

            // Fond de l'onglet : blanc si actif, pilule neutre au survol.
            let top_round = egui::CornerRadius {
                nw: 8,
                ne: 8,
                sw: 0,
                se: 0,
            };
            if active {
                ui.painter().rect_filled(rect, top_round, Color32::WHITE);
            } else if resp.hovered() || resp.is_pointer_button_down_on() {
                ui.painter()
                    .rect_filled(rect, top_round, palette::HOVER_NEUTRAL);
            }

            let mid = rect.center().y;
            ui.painter().text(
                egui::pos2(rect.left() + 12.0, mid),
                egui::Align2::LEFT_CENTER,
                tab_icon,
                egui::FontId::proportional(14.0),
                palette::TEXT,
            );
            ui.painter().text(
                egui::pos2(rect.left() + 36.0, mid),
                egui::Align2::LEFT_CENTER,
                label,
                font,
                if active {
                    palette::TEXT
                } else {
                    palette::TEXT_DIM
                },
            );

            // Bouton de fermeture : visible sur l'onglet actif et au survol.
            let close_rect = egui::Rect::from_center_size(
                egui::pos2(rect.right() - 15.0, mid),
                egui::vec2(22.0, 22.0),
            );
            let close_resp = ui.interact(
                close_rect,
                ui.id().with(("tab_close", i)),
                egui::Sense::click(),
            );
            if close_resp.hovered() || close_resp.is_pointer_button_down_on() {
                ui.painter()
                    .rect_filled(close_rect, 4.0, Color32::from_black_alpha(26));
            }
            if active || resp.hovered() || close_resp.hovered() {
                ui.painter().text(
                    close_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "×",
                    egui::FontId::proportional(14.0),
                    palette::TEXT_DIM,
                );
            }

            if resp.clicked() {
                switch = Some(i);
            }
            if resp.middle_clicked() {
                close = Some(i);
            }
            if close_resp.clicked() {
                close = Some(i);
            }
        }

        // Bouton « + » (nouvel onglet).
        ui.add_space(4.0);
        let (plus_rect, plus_resp) =
            ui.allocate_exact_size(egui::vec2(30.0, TAB_H - 6.0), egui::Sense::click());
        if plus_resp.hovered() || plus_resp.is_pointer_button_down_on() {
            ui.painter()
                .rect_filled(plus_rect, 4.0, palette::HOVER_NEUTRAL);
        }
        ui.painter().text(
            plus_rect.center(),
            egui::Align2::CENTER_CENTER,
            "+",
            egui::FontId::proportional(17.0),
            palette::TEXT,
        );
        if plus_resp.clicked() {
            new_tab = true;
        }
    });

    // Filet sous la barre d'onglets.
    let rect = ui.max_rect();
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.bottom() - 0.5),
            egui::pos2(rect.right(), rect.bottom() - 0.5),
        ],
        egui::Stroke::new(1.0, palette::BORDER),
    );

    // Actions différées (l'état ne change qu'après le rendu de la barre).
    if let Some(i) = switch {
        app.switch_tab(i);
    }
    if let Some(i) = close
        && !app.close_tab(i)
    {
        // Dernier onglet fermé : on ferme la fenêtre, comme Windows 11.
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }
    if new_tab {
        app.new_tab();
    }
}

/// Point d'entrée du rendu : assemble toute l'interface.
/// `ui` est l'Ui racine fournie par eframe ; `ctx` sert aux dialogues modaux.
pub fn show(app: &mut ExplorerApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    // Style global : thème clair garanti, popups arrondis façon Windows 11.
    ensure_style(ctx);

    // Traitement des résultats d'opérations distantes asynchrones
    // (connexion, listage, lecture) terminés sur un worker thread.
    app.process_remote_messages(ctx);

    // --- Raccourcis clavier globaux ---
    handle_shortcuts(app, ctx);

    // --- Barre d'onglets (style Windows 11) ---
    egui::Panel::top("tabs")
        .frame(
            egui::Frame::NONE
                .fill(palette::TOOLBAR_BG)
                .inner_margin(egui::Margin {
                    left: 0,
                    right: 0,
                    top: 2,
                    bottom: 0,
                }),
        )
        .show(ui, |ui| {
            tab_bar(app, ui);
        });

    // --- Ligne d'adresse : navigation, fil d'Ariane et recherche ---
    egui::Panel::top("address")
        .frame(
            egui::Frame::NONE
                .fill(palette::TOOLBAR_BG)
                .inner_margin(egui::Margin::symmetric(6, 5)),
        )
        .show(ui, |ui| {
            let action = address_bar(app, ui);
            handle_action(app, ctx, action);
        });

    // --- Barre de commandes ---
    egui::Panel::top("commandbar")
        .frame(
            egui::Frame::NONE
                .fill(palette::TOOLBAR_BG)
                .inner_margin(egui::Margin::symmetric(4, 3)),
        )
        .show(ui, |ui| {
            let action = command_bar(app, ui);
            handle_action(app, ctx, action);
        });

    // --- Panneau latéral (arborescence) ---
    egui::Panel::left("tree")
        .resizable(true)
        .default_size(240.0)
        .min_size(160.0)
        .max_size(480.0)
        .frame(egui::Frame::NONE.fill(palette::TREE_BG).inner_margin(6.0))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let action = tree_panel(app, ui, ctx);
                    handle_action(app, ctx, action);
                });
        });

    // --- Barre d'état ---
    egui::Panel::bottom("status")
        .frame(egui::Frame::NONE.fill(palette::STATUS_BG))
        .show(ui, |ui| {
            status_bar(app, ui);
        });

    // --- Panneau central : liste des fichiers ---
    // Le fond cliquable est enregistré AVANT le rendu des lignes : il a donc
    // la priorité la plus basse et seul un clic sur le vide lui arrive.
    // (En egui, un widget enregistré après les lignes avalerait tous les
    // clics et la sélection ne fonctionnerait plus.)
    let mut bg: Option<egui::Response> = None;
    egui::CentralPanel::default()
        .frame(
            egui::Frame::NONE
                .fill(palette::LIST_BG)
                .stroke(egui::Stroke::new(1.0, palette::BORDER))
                .inner_margin(6.0),
        )
        .show(ui, |ui| {
            bg = Some(ui.interact(
                ui.max_rect(),
                ui.id().with("bg_click"),
                egui::Sense::click(),
            ));
            let action = file_list(app, ui);
            handle_action(app, ctx, action);
        });

    if let Some(bg) = bg {
        // Clic sur le vide : tout désélectionner.
        if bg.clicked() {
            app.tab_mut().selected.clear();
            app.update_status();
        }
        // Clic droit sur le vide : menu contextuel style Windows 11.
        let mut action = Action::None;
        bg.context_menu(|ui| {
            background_context_menu(app, ui, &mut action);
        });
        handle_action(app, ctx, action);
    }

    // --- Dialogues modaux ---
    // Déverrouillage du mot de passe maître (requis avant toute connexion distante).
    if app.master_password.is_none() && app.tab().remote_session.is_none() {
        app.unlock_dialog.open = true;
    }
    if app.unlock_dialog.open {
        show_unlock_dialog(app, ctx);
    }
    // Connexion à un serveur distant.
    if app.remote_dialog.open {
        show_remote_connect_dialog(app, ctx);
    }
    show_dialog(app, ctx);

    // --- Notification animée (toast) et nettoyage du surlignage expiré ---
    render_toast(app, ctx);
    if app
        .tab()
        .highlight
        .as_ref()
        .is_some_and(|(_, at)| at.elapsed().as_secs_f32() >= HIGHLIGHT_SECS)
    {
        app.tab_mut().highlight = None;
    }
}

/// Applique une action issue de l'UI sur l'état de l'application.
fn handle_action(app: &mut ExplorerApp, ctx: &egui::Context, action: Action) {
    match action {
        Action::None => {}
        Action::Navigate(path) => app.navigate(path),
        Action::Back => app.go_back(),
        Action::Forward => app.go_forward(),
        Action::Up => app.go_up(),
        Action::Refresh => app.refresh(),
        Action::Open(path) => app.open_entry(ctx, &path),
        Action::NewFolder => {
            app.dialog = Dialog::NewFolder {
                name: "Nouveau dossier".to_string(),
            };
        }
        Action::NewFile => {
            app.dialog = Dialog::NewFile {
                name: "nouveau_fichier.txt".to_string(),
            };
        }
        Action::Rename | Action::RenameSel => app.rename_selected(),
        Action::Delete => {
            app.dialog = Dialog::ConfirmDelete {
                paths: app.tab().selected.iter().cloned().collect(),
            };
        }
        Action::Copy => app.copy_selection(false),
        Action::Cut => app.copy_selection(true),
        Action::Paste => app.paste(),
        Action::PasteIn(path) => app.paste_in(&path),
        Action::CopyPath(_) => {}
        Action::ToggleHidden => {
            app.tab_mut().show_hidden = !app.tab().show_hidden;
            app.refresh();
        }
    }
}

/// Gère les raccourcis clavier globaux.
fn handle_shortcuts(app: &mut ExplorerApp, ctx: &egui::Context) {
    // Tant qu'un dialogue est ouvert : Échap le ferme, et les autres raccourcis
    // globaux sont suspendus (sinon Backspace/Suppr/F2 agiraient pendant la saisie).
    if !matches!(app.dialog, Dialog::None) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.dialog = Dialog::None;
        }
        return;
    }
    // Ne pas détourner les touches pendant la saisie dans un champ texte
    // (recherche, adresse…), sinon Backspace remonterait d'un dossier, etc.
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let mods = ctx.input(|i| i.modifiers);
    let ctrl = mods.ctrl || mods.command;

    ctx.input(|i| {
        // F5 : actualiser.
        if i.key_pressed(egui::Key::F5) {
            app.refresh();
        }
        // Retour arrière (touche Backspace) : dossier parent.
        if i.key_pressed(egui::Key::Backspace) {
            app.go_up();
        }
        // Alt + Gauche / Droite : navigation.
        if mods.alt && i.key_pressed(egui::Key::ArrowLeft) {
            app.go_back();
        }
        if mods.alt && i.key_pressed(egui::Key::ArrowRight) {
            app.go_forward();
        }
        if ctrl {
            if i.key_pressed(egui::Key::A) {
                app.select_all();
            }
            if i.key_pressed(egui::Key::C) {
                app.copy_selection(false);
            }
            if i.key_pressed(egui::Key::X) {
                app.copy_selection(true);
            }
            if i.key_pressed(egui::Key::V) {
                app.paste();
            }
            if i.key_pressed(egui::Key::F) {
                // Le focus recherche est géré par le champ lui-même.
            }
            // Ctrl+1/2/3 : modes d'affichage (Liste / Détail / Miniatures).
            if i.key_pressed(egui::Key::Num1) {
                app.tab_mut().view_mode = ViewMode::List;
            }
            if i.key_pressed(egui::Key::Num2) {
                app.tab_mut().view_mode = ViewMode::Details;
            }
            if i.key_pressed(egui::Key::Num3) {
                app.tab_mut().view_mode = ViewMode::Thumbnails;
            }
        }
        // Suppr : mettre à la corbeille (uniquement avec une sélection).
        if i.key_pressed(egui::Key::Delete) && !app.tab().selected.is_empty() {
            app.dialog = Dialog::ConfirmDelete {
                paths: app.tab().selected.iter().cloned().collect(),
            };
        }
        // F2 : renommer.
        if i.key_pressed(egui::Key::F2) {
            app.rename_selected();
        }
    });
}
