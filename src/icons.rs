//! Icônes système réelles (macOS : `NSWorkspace.iconForFile:`) et vignettes
//! d'aperçu des images, avec cache de textures egui.
//!
//! Le travail coûteux (appels Objective-C, décodage/redimensionnement
//! d'image) est étalé par budget de frame : l'icône d'un élément visible est
//! donc prête au bout d'une frame ou deux, sans geler le rendu.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use eframe::egui;

/// Nouvelles icônes système créées au plus par frame (appels Objective-C).
const MAX_ICON_LOADS: u32 = 32;
/// Nouvelles vignettes d'aperçu décodées au plus par frame (décodage disque).
const MAX_THUMB_LOADS: u32 = 4;

/// Textures d'un même chemin, indexées par taille demandée (pixels).
type BySize = HashMap<u32, egui::TextureHandle>;

/// Cache d'icônes système et de vignettes : (chemin, taille) → texture.
pub struct IconCache {
    /// Icônes OS par chemin, puis par taille.
    icons: HashMap<PathBuf, BySize>,
    /// Vignettes d'aperçu (fichiers image) par chemin, puis par taille.
    thumbs: HashMap<PathBuf, BySize>,
    /// Chemins dont l'icône OS a échoué (non retentés à l'infini).
    icon_failed: HashSet<PathBuf>,
    /// Chemins dont le décodage d'image a échoué (non retentés).
    thumb_failed: HashSet<PathBuf>,
    /// `i.time` de la frame où les budgets ont été remis à zéro.
    last_time: f64,
    /// Icônes restantes à charger dans la frame courante.
    icon_budget: u32,
    /// Vignettes restantes à décoder dans la frame courante.
    thumb_budget: u32,
}

impl IconCache {
    pub fn new() -> Self {
        Self {
            icons: HashMap::new(),
            thumbs: HashMap::new(),
            icon_failed: HashSet::new(),
            thumb_failed: HashSet::new(),
            last_time: -1.0,
            icon_budget: 0,
            thumb_budget: 0,
        }
    }

    /// Consomme une unité du budget correspondant ; `false` si la frame est
    /// déjà saturée (l'appelant affiche alors son emoji de repli).
    fn take(&mut self, ctx: &egui::Context, thumb: bool) -> bool {
        let now = ctx.input(|i| i.time);
        if now != self.last_time {
            self.last_time = now;
            self.icon_budget = 0;
            self.thumb_budget = 0;
        }
        let limit = if thumb {
            MAX_THUMB_LOADS
        } else {
            MAX_ICON_LOADS
        };
        let budget = if thumb {
            &mut self.thumb_budget
        } else {
            &mut self.icon_budget
        };
        if *budget >= limit {
            // La file d'attente n'est pas vide : redessiner pour la vider.
            ctx.request_repaint();
            return false;
        }
        *budget += 1;
        // Un chargement vient d'être autorisé : au moins une frame suivante
        // est utile pour vider le reste de la file sans attendre un geste de
        // l'utilisateur (sinon les icônes suivantes resteraient invisibles).
        ctx.request_repaint();
        true
    }

    /// Icône système de `path` à `px` pixels (déjà en cache la plupart du
    /// temps). `None` si l'OS n'a rien fourni ou si le budget de la frame est
    /// épuisé : l'appelant affiche alors l'emoji de repli.
    pub fn system(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
        px: u32,
    ) -> Option<egui::TextureHandle> {
        if let Some(handle) = self.icons.get(path).and_then(|by| by.get(&px)) {
            return Some(handle.clone());
        }
        if self.icon_failed.contains(path) || !self.take(ctx, false) {
            return None;
        }
        let Some(natural) = os_icon_image(path) else {
            self.icon_failed.insert(path.to_path_buf());
            return None;
        };
        let handle = ctx.load_texture(
            format!("icon:{}@{px}", path.display()),
            resize_pad(&natural, px),
            egui::TextureOptions::LINEAR,
        );
        self.icons
            .entry(path.to_path_buf())
            .or_default()
            .insert(px, handle.clone());
        Some(handle)
    }

    /// Vignette d'aperçu du fichier image `path` à `px` pixels (carrée, sans
    /// déformation, fond transparent) ; `None` si le fichier n'est pas
    /// décodable (échec mémorisé, non retenté à l'infini).
    pub fn thumbnail(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
        px: u32,
    ) -> Option<egui::TextureHandle> {
        if let Some(handle) = self.thumbs.get(path).and_then(|by| by.get(&px)) {
            return Some(handle.clone());
        }
        if self.thumb_failed.contains(path) || !self.take(ctx, true) {
            return None;
        }
        // Images classiques décodées par `image` ; PDF rendus par PDFKit.
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .unwrap_or_default();
        let decoded = if is_pdf_file(&ext) {
            pdf_first_page(path)
        } else {
            decode_rgba(path)
        };
        let Some(decoded) = decoded else {
            self.thumb_failed.insert(path.to_path_buf());
            return None;
        };
        let handle = ctx.load_texture(
            format!("thumb:{}@{px}", path.display()),
            resize_pad(&decoded, px),
            egui::TextureOptions::LINEAR,
        );
        self.thumbs
            .entry(path.to_path_buf())
            .or_default()
            .insert(px, handle.clone());
        Some(handle)
    }
}

impl Default for IconCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Extensions que le crate `image` sait décoder (vignettes).
pub fn is_image_file(extension: &str) -> bool {
    matches!(
        extension,
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "tiff" | "tif"
    )
}

/// Extension PDF (aperçu de la première page dans la vue Miniatures).
pub fn is_pdf_file(extension: &str) -> bool {
    extension == "pdf"
}

/// Fichiers pour lesquels la vue Miniatures affiche une vraie vignette
/// (images décodables + première page des PDF) ; sinon icône système.
pub fn is_previewable_file(extension: &str) -> bool {
    is_image_file(extension) || is_pdf_file(extension)
}

/// Décode `path` en image RGBA (format deviné par le contenu : robuste à la
/// casse de l'extension). `None` si illisible ou image vide.
fn decode_rgba(path: &Path) -> Option<image::RgbaImage> {
    let reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let img = reader.decode().ok()?.to_rgba8();
    if img.width() == 0 || img.height() == 0 {
        return None;
    }
    Some(img)
}

/// Redimensionne `img` pour tenir dans `px × px` sans déformation, centrée
/// sur un carré transparent (comme les vignettes de l'Explorateur).
fn resize_pad(img: &image::RgbaImage, px: u32) -> egui::ColorImage {
    let side = px as usize;
    let mut canvas = vec![egui::Color32::TRANSPARENT; side * side];
    if img.width() > 0 && img.height() > 0 {
        // Redimensionnement à la « plus proche » façon vignettes (rapide et
        // suffisant pour des aperçus de 96 px max).
        let small = image::imageops::thumbnail(img, px, px);
        let ox = ((px - small.width()) / 2) as usize;
        let oy = ((px - small.height()) / 2) as usize;
        for (y, row) in small.rows().enumerate() {
            let base = (oy + y) * side + ox;
            for (x, p) in row.enumerate() {
                canvas[base + x] = egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]);
            }
        }
    }
    egui::ColorImage::new([side, side], canvas)
}

/// Icône système macOS : `NSWorkspace.iconForFile:` → `NSImage` →
/// représentation TIFF → décodage. `None` si le chemin n'est pas en UTF-8 ou
/// si l'OS n'a rien renvoyé.
#[cfg(target_os = "macos")]
fn os_icon_image(path: &Path) -> Option<image::RgbaImage> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::NSString;

    let ns_path = NSString::from_str(path.to_str()?);
    let image = NSWorkspace::sharedWorkspace().iconForFile(&ns_path);
    let tiff = image.TIFFRepresentation()?;
    Some(image::load_from_memory(&tiff.to_vec()).ok()?.to_rgba8())
}

/// Hors macOS : pas d'icône OS native (emoji de repli côté interface).
#[cfg(not(target_os = "macos"))]
fn os_icon_image(_path: &Path) -> Option<image::RgbaImage> {
    None
}

/// Aperçu PDF : rendu **natif** de la première page via PDFKit
/// (`PDFDocument` → page 1 → `thumbnail(of:for:)` → NSImage → TIFF → image).
/// `None` si le chemin n'est pas en UTF-8, si le document est verrouillé ou
/// illisible (l'échec est mémorisé par `IconCache::thumbnail`).
#[cfg(target_os = "macos")]
fn pdf_first_page(path: &Path) -> Option<image::RgbaImage> {
    use objc2::AnyThread;
    use objc2_foundation::{NSString, NSURL};
    use objc2_pdf_kit::{PDFDisplayBox, PDFDocument};

    let url = NSURL::fileURLWithPath(&NSString::from_str(path.to_str()?));
    let doc = unsafe { PDFDocument::initWithURL(PDFDocument::alloc(), &url)? };
    if unsafe { doc.isLocked() } {
        return None;
    }
    let page = unsafe { doc.pageAtIndex(0) }?;
    // Demande ~256 px (≈ 2,7× la vignette 96 px) : net sur écran Retina.
    let image = unsafe { page.thumbnailOfSize_forBox(NS_SIZE_256, PDFDisplayBox::CropBox) };
    let tiff = image.TIFFRepresentation()?;
    Some(image::load_from_memory(&tiff.to_vec()).ok()?.to_rgba8())
}

/// Taille de rendu des aperçus PDF (largest côté de la vignette × ~2,7).
#[cfg(target_os = "macos")]
const NS_SIZE_256: objc2_foundation::NSSize = objc2_foundation::NSSize {
    width: 256.0,
    height: 256.0,
};

/// Hors macOS : pas de PDFKit (icône système de repli côté interface).
#[cfg(not(target_os = "macos"))]
fn pdf_first_page(_path: &Path) -> Option<image::RgbaImage> {
    None
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use std::path::Path;

    /// Écrit un PDF minimal valide (1 page, xref avec offsets calculés).
    fn write_test_pdf(path: &Path) {
        let stream = "BT /F1 36 Tf 40 80 Td (Preview PDF) Tj ET";
        let objs = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] \
             /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
                .to_string(),
            format!(
                "<< /Length {} >>\nstream\n{stream}\nendstream",
                stream.len()
            ),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        ];
        let mut body = String::from("%PDF-1.4\n");
        let mut offsets = Vec::with_capacity(objs.len());
        for (i, obj) in objs.iter().enumerate() {
            offsets.push(body.len());
            body.push_str(&format!("{} 0 obj\n{obj}\nendobj\n", i + 1));
        }
        let xref_pos = body.len();
        body.push_str(&format!("xref\n0 {}\n", objs.len() + 1));
        body.push_str("0000000000 65535 f \n");
        for off in &offsets {
            body.push_str(&format!("{off:010} 00000 n \n"));
        }
        body.push_str(&format!(
            "trailer\n<< /Root 1 0 R /Size {} >>\nstartxref\n{xref_pos}\n%%EOF\n",
            objs.len() + 1
        ));
        std::fs::write(path, body).expect("écriture du PDF de test");
    }

    #[test]
    fn pdf_first_page_rend_sans_panique() {
        let pdf = std::env::temp_dir().join("smart_explorer_pdf_test.pdf");
        write_test_pdf(&pdf);
        let img = super::pdf_first_page(&pdf).expect("PDFKit doit rendre la 1re page");
        assert!(img.width() > 0 && img.height() > 0, "image vide");
    }
}
