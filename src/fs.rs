//! Opérations sur le système de fichiers, multiplateformes
//! (Windows, Linux, macOS).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};

/// Représente un élément (fichier ou dossier) listé dans un répertoire.
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_hidden: bool,
    /// Taille en octets (0 pour les dossiers).
    pub size: u64,
    pub modified: Option<DateTime<Local>>,
    /// Extension en minuscules, sans le point (vide si aucune).
    pub extension: String,
    /// Symlink ?
    pub is_symlink: bool,
}

impl Entry {
    /// Construit une `Entry` à partir d'un chemin. Retourne `None` si les
    /// métadonnées sont illisibles.
    pub fn from_path(path: &Path) -> Option<Self> {
        // `symlink_metadata` ne suit pas les liens : on détecte ainsi les symlinks.
        let meta = fs::symlink_metadata(path).ok()?;
        let file_type = meta.file_type();

        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => path.to_string_lossy().to_string(),
        };

        let is_dir = file_type.is_dir();
        let is_symlink = file_type.is_symlink();

        let extension = if is_dir {
            String::new()
        } else {
            path.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        };

        let size = if is_dir { 0 } else { meta.len() };
        let modified = meta.modified().ok().map(DateTime::<Local>::from);
        let is_hidden = is_hidden(&meta, &name);

        Some(Entry {
            path: path.to_path_buf(),
            name,
            is_dir,
            is_hidden,
            size,
            modified,
            extension,
            is_symlink,
        })
    }

    /// Libellé lisible du type d'élément (colonne "Type").
    pub fn type_label(&self) -> String {
        if self.is_dir {
            return "Dossier de fichiers".to_string();
        }
        match self.extension.as_str() {
            "" => "Fichier".to_string(),
            ext => format!("Fichier {}", ext.to_uppercase()),
        }
    }
}

/// Détecte si un élément est caché, selon l'OS.
#[cfg(windows)]
fn is_hidden(meta: &fs::Metadata, name: &str) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    name.starts_with('.') || (meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
}

#[cfg(not(windows))]
fn is_hidden(_meta: &fs::Metadata, name: &str) -> bool {
    name.starts_with('.')
}

/// Liste le contenu d'un répertoire.
/// `show_hidden` : inclure ou non les éléments cachés.
pub fn list_dir(path: &Path, show_hidden: bool) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(e) = Entry::from_path(&path) {
            if !show_hidden && e.is_hidden {
                continue;
            }
            entries.push(e);
        }
    }
    Ok(entries)
}

/// Liste uniquement les sous-dossiers (pour l'arborescence latérale).
pub fn list_subdirs(path: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(rd) = fs::read_dir(path) {
        for entry in rd.flatten() {
            let p = entry.path();
            if let Ok(meta) = fs::metadata(&p)
                && meta.is_dir()
            {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !name.starts_with('.') {
                    dirs.push(p);
                }
            }
        }
    }
    dirs.sort_by(|a, b| {
        let an = a
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let bn = b
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        an.cmp(&bn)
    });
    dirs
}

/// Retourne les "racines" à afficher dans l'arborescence :
/// les lecteurs sous Windows, `/` ailleurs.
pub fn get_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let mut roots = Vec::new();
        for c in b'A'..=b'Z' {
            let p = PathBuf::from(format!("{}:\\", c as char));
            if p.exists() {
                roots.push(p);
            }
        }
        if roots.is_empty() {
            roots.push(PathBuf::from("C:\\"));
        }
        roots
    }
    #[cfg(not(windows))]
    {
        vec![PathBuf::from("/")]
    }
}

/// Répertoire "home" de l'utilisateur.
pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Crée un nouveau dossier.
pub fn create_dir(path: &Path) -> io::Result<()> {
    fs::create_dir(path)
}

/// Crée un nouveau fichier vide.
pub fn create_file(path: &Path) -> io::Result<()> {
    fs::File::create(path).map(|_| ())
}

/// Renomme / déplace un élément.
pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

/// Supprime définitivement (sans corbeille) — utilisé en secours.
pub fn remove_permanently(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Copie récursivement un fichier ou un dossier.
pub fn copy_path(from: &Path, to: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(from)?;
    if meta.is_dir() {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            let src = entry.path();
            let dst = to.join(entry.file_name());
            copy_path(&src, &dst)?;
        }
        Ok(())
    } else {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
        Ok(())
    }
}

/// Déplace un élément (rename, avec repli copier+supprimer si changement de
/// périphérique, ce qui arrive entre disques).
pub fn move_path(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_path(from, to)?;
            remove_permanently(from)
        }
    }
}

/// Sépare un nom de fichier en (base, extension_avec_point).
/// `"archive.tar.gz"` -> `("archive.tar", ".gz")`, `"notes"` -> `("notes", "")`.
pub fn split_name(name: &str) -> (String, String) {
    match name.rfind('.') {
        Some(idx) if idx > 0 => (name[..idx].to_string(), name[idx..].to_string()),
        _ => (name.to_string(), String::new()),
    }
}

/// Calcule un chemin de destination unique dans `dest_dir` pour `name`
/// (ex. `"notes (1).txt"` si `"notes.txt"` existe déjà).
pub fn unique_dest(dest_dir: &Path, name: &str) -> PathBuf {
    let candidate = dest_dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = split_name(name);
    for i in 1..10_000 {
        let candidate = dest_dir.join(format!("{stem} ({i}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dest_dir.join(name)
}

/// Ouvre un fichier/dossier avec l'application par défaut du système.
pub fn open_with_default(path: &Path) -> io::Result<()> {
    open::that(path)
}

/// Met à la corbeille (multiplateforme : Corbeille Windows, Poubelle macOS/Linux).
pub fn move_to_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| e.to_string())
}

/// Formate une taille en octets de façon lisible (Ko, Mo, Go…).
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["o", "Ko", "Mo", "Go", "To", "Po"];
    if bytes < 1024 {
        return format!("{bytes} o");
    }
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", size, UNITS[unit])
}
