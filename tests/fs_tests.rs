//! Tests d'intégration pour le module `fs` (opérations sur le système de fichiers).
//!
//! Ces tests créent un répertoire temporaire isolé, exercent les opérations
//! (création, copie, déplacement, renommage, suppression, listage, tri…)
//! puis nettoient. Ils sont multiplateformes.

use std::fs;
use std::path::PathBuf;

use smart_explorer::fs as sefs;

/// Crée un dossier temporaire unique pour un test.
fn temp_dir(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let unique = format!(
        "smart_explorer_test_{}_{}_{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    dir.push(unique);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn format_size_lisible() {
    assert_eq!(sefs::format_size(0), "0 o");
    assert_eq!(sefs::format_size(512), "512 o");
    assert_eq!(sefs::format_size(1024), "1.0 Ko");
    assert_eq!(sefs::format_size(1024 * 1024), "1.0 Mo");
    assert_eq!(sefs::format_size(1536), "1.5 Ko");
}

#[test]
fn split_name_fonctionne() {
    assert_eq!(
        sefs::split_name("notes.txt"),
        ("notes".into(), ".txt".into())
    );
    assert_eq!(
        sefs::split_name("archive.tar.gz"),
        ("archive.tar".into(), ".gz".into())
    );
    assert_eq!(
        sefs::split_name("sans_extension"),
        ("sans_extension".into(), "".into())
    );
    assert_eq!(
        sefs::split_name(".gitignore"),
        (".gitignore".into(), "".into())
    );
}

#[test]
fn creer_dossier_et_fichier() {
    let dir = temp_dir("create");
    let sub = dir.join("mon_dossier");
    sefs::create_dir(&sub).unwrap();
    assert!(sub.is_dir());

    let file = dir.join("mon_fichier.txt");
    sefs::create_file(&file).unwrap();
    assert!(file.is_file());

    sefs::remove_permanently(&dir).ok();
}

#[test]
fn list_dir_filtre_caches() {
    let dir = temp_dir("list");
    sefs::create_file(&dir.join("visible.txt")).unwrap();
    sefs::create_file(&dir.join(".cache_secret")).unwrap();
    sefs::create_dir(&dir.join("sous_dossier")).unwrap();

    let sans_caches = sefs::list_dir(&dir, false).unwrap();
    assert_eq!(
        sans_caches.len(),
        2,
        "les éléments cachés doivent être exclus"
    );

    let avec_caches = sefs::list_dir(&dir, true).unwrap();
    assert_eq!(avec_caches.len(), 3, "tous les éléments doivent apparaître");

    sefs::remove_permanently(&dir).ok();
}

#[test]
fn entry_contient_metadonnees() {
    let dir = temp_dir("meta");
    let file = dir.join("data.bin");
    fs::write(&file, b"hello world").unwrap();

    let entries = sefs::list_dir(&dir, true).unwrap();
    let entry = entries.iter().find(|e| e.name == "data.bin").unwrap();
    assert!(!entry.is_dir);
    assert_eq!(entry.size, 11);
    assert_eq!(entry.extension, "bin");
    assert!(!entry.is_hidden);
    assert!(entry.modified.is_some());

    sefs::remove_permanently(&dir).ok();
}

#[test]
fn copie_recursive() {
    let dir = temp_dir("copy");
    let src = dir.join("source");
    sefs::create_dir(&src).unwrap();
    fs::write(src.join("a.txt"), b"contenu a").unwrap();
    sefs::create_dir(&src.join("nested")).unwrap();
    fs::write(src.join("nested/b.txt"), b"contenu b").unwrap();

    let dest = dir.join("destination");
    sefs::copy_path(&src, &dest).unwrap();

    assert!(dest.join("a.txt").is_file());
    assert!(dest.join("nested/b.txt").is_file());
    assert_eq!(
        fs::read_to_string(dest.join("nested/b.txt")).unwrap(),
        "contenu b"
    );
    // La source existe toujours.
    assert!(src.join("a.txt").is_file());

    sefs::remove_permanently(&dir).ok();
}

#[test]
fn deplace_element() {
    let dir = temp_dir("move");
    let file = dir.join("a_deplacer.txt");
    fs::write(&file, b"data").unwrap();

    let sub = dir.join("cible");
    sefs::create_dir(&sub).unwrap();
    let dest = sub.join("a_deplacer.txt");

    sefs::move_path(&file, &dest).unwrap();
    assert!(!file.exists(), "la source ne doit plus exister");
    assert!(dest.is_file(), "la destination doit exister");

    sefs::remove_permanently(&dir).ok();
}

#[test]
fn renomme_element() {
    let dir = temp_dir("rename");
    let file = dir.join("ancien.txt");
    fs::write(&file, b"x").unwrap();

    let nouveau = dir.join("nouveau.txt");
    sefs::rename(&file, &nouveau).unwrap();
    assert!(!file.exists());
    assert!(nouveau.is_file());

    sefs::remove_permanently(&dir).ok();
}

#[test]
fn unique_dest_evite_collision() {
    let dir = temp_dir("unique");
    fs::write(dir.join("doc.txt"), b"1").unwrap();

    let dest = sefs::unique_dest(&dir, "doc.txt");
    assert_eq!(dest.file_name().unwrap(), "doc (1).txt");
    assert!(!dest.exists());

    fs::write(dir.join("doc (1).txt"), b"2").unwrap();
    let dest2 = sefs::unique_dest(&dir, "doc.txt");
    assert_eq!(dest2.file_name().unwrap(), "doc (2).txt");

    sefs::remove_permanently(&dir).ok();
}

#[test]
fn list_subdirs_ignore_fichiers_et_caches() {
    let dir = temp_dir("subdirs");
    sefs::create_dir(&dir.join("alpha")).unwrap();
    sefs::create_dir(&dir.join("beta")).unwrap();
    fs::write(dir.join("fichier.txt"), b"x").unwrap();
    sefs::create_dir(&dir.join(".hidden_dir")).unwrap();

    let subdirs = sefs::list_subdirs(&dir);
    let names: Vec<String> = subdirs
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert_eq!(subdirs.len(), 2);
    assert!(names.contains(&"alpha".to_string()));
    assert!(names.contains(&"beta".to_string()));
    assert!(!names.iter().any(|n| n.starts_with('.')));

    sefs::remove_permanently(&dir).ok();
}

#[test]
fn racines_non_vides() {
    let roots = sefs::get_roots();
    assert!(
        !roots.is_empty(),
        "il doit toujours y avoir au moins une racine"
    );
}
