//! Animations façon macOS : fluides, ease-out et discrètes.
//!
//! egui est en mode immédiat : il n'y a pas d'arbre d'objets dont on anime
//! directement les propriétés. On stocke donc les *cibles* des animations et
//! on laisse `ctx.animate_bool_id` interpoler une valeur en [0.0, 1.0] à
//! chaque frame (avec easing intégré et repaint automatique).
//!
//! Chaque `id` doit être unique par élément animé (bouton, dialog, toast…).

use eframe::egui;

/// Durée d'une transition standard (macOS ≈ 0.25 s).
pub const DURATION: f32 = 0.25;
/// Durée d'un micro-feedback (bouton, surlignage…) (≈ 0.15 s).
/// Réservée aux futures animations de boutons / surlignages.
#[allow(dead_code)]
pub const FEEDBACK: f32 = 0.15;

/// Valeur interpolée [0.0, 1.0] pour un booléen animé.
///
/// Quand `target` passe de `false` à `true`, la valeur glisse de 0 à 1 (et
/// inversement). egui demande automatiquement un repaint tant que l'interpolation
/// n'est pas terminée.
pub fn bool_val(ctx: &egui::Context, id: impl Into<egui::Id>, target: bool) -> f32 {
    ctx.animate_bool(id.into(), target)
}

/// Opacité [0.0, 1.0] d'un élément qui apparaît ou disparaît.
pub fn fade(ctx: &egui::Context, id: impl Into<egui::Id>, visible: bool) -> f32 {
    bool_val(ctx, id, visible) as f32
}

/// Décalage horizontal de sortie (élément qui glisse vers la droite avant
/// de disparaître). Retourne le décalage en pixels (0 → `max_offset`).
///
/// Utilitaire prêt à l'emploi pour animer la suppression d'un élément.
#[allow(dead_code)]
pub fn slide_out_x(
    ctx: &egui::Context,
    id: impl Into<egui::Id>,
    leaving: bool,
    max_offset: f32,
) -> f32 {
    let t = bool_val(ctx, id, leaving) as f32;
    // Ease-out : démarre vite, puis se stabilise.
    let eased = 1.0 - (1.0 - t).powi(3);
    max_offset * eased
}

/// Décalage vertical d'entrée (élément qui monte depuis le bas).
/// Retourne le décalage en pixels (`max_offset` → 0).
///
/// Utilitaire prêt à l'emploi pour animer l'apparition d'un élément.
#[allow(dead_code)]
pub fn slide_in_y(
    ctx: &egui::Context,
    id: impl Into<egui::Id>,
    entering: bool,
    max_offset: f32,
) -> f32 {
    let t = bool_val(ctx, id, entering) as f32;
    let eased = 1.0 - (1.0 - t).powi(3);
    max_offset * (1.0 - eased)
}

/// Applique une transparence à une couleur.
pub fn with_alpha(color: egui::Color32, alpha: f32) -> egui::Color32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
    egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), a)
}

/// Dessine un fond sombre plein écran (overlay de dialog macOS).
/// Retourne l'opacité animée [0.0, 1.0].
pub fn dialog_overlay(ctx: &egui::Context, visible: bool) -> f32 {
    let alpha = fade(ctx, egui::Id::new("smart_explorer/dialog_overlay"), visible);
    if alpha > 0.0 {
        let screen_rect = ctx
            .input(|i| i.raw.screen_rect)
            .unwrap_or(egui::Rect {
                min: Default::default(),
                max: Default::default(),
            });
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(
            screen_rect,
            0.0,
            with_alpha(egui::Color32::BLACK, alpha * 0.45_f32),
        );
    }
    alpha
}
