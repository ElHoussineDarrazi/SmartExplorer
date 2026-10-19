//! État de l'application et logique de l'explorateur de fichiers.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Instant;

use crate::animation;
use crate::credentials;
use crate::fs;
use crate::remote;
use eframe::egui;

/// Colonne de tri.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Size,
    Modified,
    Type,
}

/// Sens du tri.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Contenu du presse-papiers interne (copier / couper).
#[derive(Clone)]
pub struct Clipboard {
    pub paths: Vec<PathBuf>,
    pub cut: bool,
}

/// Notification temporaire affichée en overlay, avec icône animée.
pub struct Toast {
    pub icon: String,
    pub message: String,
    pub created_at: Instant,
    pub lifetime: f32,
}

/// Action demandée à l'UI (résolue après le rendu, car elle mute l'état).
#[allow(dead_code)]
pub enum Action {
    None,
    Navigate(PathBuf),
    Back,
    Forward,
    Up,
    Refresh,
    Open(PathBuf),
    NewFolder,
    NewFile,
    Rename,
    Delete,
    Copy,
    Cut,
    Paste,
    PasteIn(PathBuf),
    CopyPath(PathBuf),
    ToggleHidden,
    RenameSel,
}

/// Direction d'une navigation, utilisée pour animer le glissement du contenu.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavDir {
    In,      // entrer dans un dossier (contenu glisse vers la gauche)
    Out,     // remonter d'un niveau (contenu glisse vers la droite)
    Back,    // retour dans l'historique (glisse vers la droite)
    Forward, // avancer dans l'historique (glisse vers la gauche)
}

/// Boîte de dialogue modale en cours.
pub enum Dialog {
    None,
    NewFolder {
        name: String,
    },
    NewFile {
        name: String,
    },
    Rename {
        path: PathBuf,
        name: String,
    },
    ConfirmDelete {
        paths: Vec<PathBuf>,
    },
    Properties {
        path: PathBuf,
    },
    SystemProperties,
    /// Carte des tailles de fichiers (carrés proportionnels à la taille).
    #[allow(dead_code)]
    FileSizeMap {
        min_size: u64,
    },
    About,
}

/// Mode d'affichage du contenu du dossier.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    List,
    Details,
    Thumbnails,
}

/// Résultat d'une opération distante asynchrone (renvoyé via un canal mpsc).
pub enum RemoteMsg {
    Connected(remote::RemoteSession),
    Listed(Vec<remote::RemoteEntry>),
    FileRead(Vec<u8>),
}

/// Opération distante en cours, sert à router le résultat asynchrone.
pub enum RemoteAwaiting {
    Connect,
    ListDir,
    ReadFile,
}

/// État éphémère de la boîte de dialogue de connexion à un serveur distant.
#[derive(Default)]
pub struct RemoteDialogState {
    pub open: bool,
    pub protocol: remote::RemoteProtocol,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub save: bool,
}

/// État éphémère du déverrouillage du magasin d'identifiants (mot de passe maître).
#[derive(Default)]
pub struct UnlockDialogState {
    pub open: bool,
    pub password: String,
}
/// Type de système exposé par un onglet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabKind {
    #[default]
    Local,
    Remote,
}

/// État de navigation isolé d'un onglet.
///
/// Chaque onglet possède son propre dossier courant, sa sélection, son
/// historique, ses filtres et, le cas échéant, sa session distante. Un
/// changement d'onglet bascule d'un système à l'autre sans les mélanger.
pub struct TabState {
    /// Type de système (local ou distant).
    pub kind: TabKind,
    /// Libellé affiché dans la barre d'onglets.
    pub label: String,
    /// Pour un onglet distant : configuration permettant de se reconnecter.
    pub remote_config: Option<remote::RemoteConfig>,
    /// Dossier courant (chemin local, ou chemin distant synthétique "/").
    pub cwd: PathBuf,
    /// Historique de navigation (pour retour / avancer).
    pub history: Vec<PathBuf>,
    pub history_index: usize,
    /// Éléments du répertoire courant (triés).
    pub entries: Vec<fs::Entry>,
    /// Chemins sélectionnés (plusieurs possibles).
    pub selected: HashSet<PathBuf>,
    /// Filtre de recherche (nom partiel).
    pub search: String,
    /// Recherche récursive dans les sous-dossiers.
    pub search_recursive: bool,
    pub show_hidden: bool,
    pub sort_column: SortColumn,
    pub sort_order: SortOrder,
    /// Dossiers dépliés dans l'arborescence.
    pub expanded: HashSet<PathBuf>,
    /// Élément en cours de renommage inline (chemin).
    pub renaming: Option<PathBuf>,
    /// Buffer de texte pour le renommage inline.
    pub rename_buffer: String,
    /// Résultats de recherche récursive.
    pub search_results: Vec<fs::Entry>,
    /// Ancre de sélection (dernier élément cliqué, pour Shift).
    pub selection_anchor: Option<PathBuf>,
    /// Message de la barre d'état.
    pub status: String,
    /// Dernière erreur à afficher.
    pub error: Option<String>,
    /// Élément à surligner après une action : (chemin, instant du flash).
    pub highlight: Option<(PathBuf, Instant)>,
    /// Mode d'affichage du contenu (Détail / Liste / Miniatures).
    pub view_mode: ViewMode,
    /// Session distante active pour cet onglet (None = navigation locale).
    pub remote_session: Option<remote::RemoteSession>,
}

impl TabState {
    /// Crée l'état d'un onglet local à partir d'un dossier.
    fn new_local(cwd: PathBuf) -> Self {
        let label = cwd
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| cwd.to_string_lossy().to_string());
        Self {
            kind: TabKind::Local,
            label,
            remote_config: None,
            cwd: cwd.clone(),
            history: vec![cwd.clone()],
            history_index: 0,
            entries: Vec::new(),
            selected: HashSet::new(),
            search: String::new(),
            search_recursive: false,
            show_hidden: false,
            sort_column: SortColumn::Name,
            sort_order: SortOrder::Asc,
            expanded: HashSet::new(),
            renaming: None,
            rename_buffer: String::new(),
            search_results: Vec::new(),
            selection_anchor: None,
            status: String::new(),
            error: None,
            highlight: None,
            view_mode: ViewMode::Details,
            remote_session: None,
        }
    }

    /// Crée l'état d'un onglet distant à partir d'une session connectée.
    fn new_remote(config: remote::RemoteConfig, session: remote::RemoteSession) -> Self {
        let label = config.display_address();
        let cwd = PathBuf::from(session.cwd.trim_end_matches('/'));
        let entries: Vec<fs::Entry> = session
            .entries
            .iter()
            .map(|re| ExplorerApp::remote_entry_to_entry(re, &session.cwd))
            .collect();
        Self {
            kind: TabKind::Remote,
            label,
            remote_config: Some(config),
            cwd: cwd.clone(),
            history: vec![cwd],
            history_index: 0,
            entries,
            selected: HashSet::new(),
            search: String::new(),
            search_recursive: false,
            show_hidden: false,
            sort_column: SortColumn::Name,
            sort_order: SortOrder::Asc,
            expanded: HashSet::new(),
            renaming: None,
            rename_buffer: String::new(),
            search_results: Vec::new(),
            selection_anchor: None,
            status: String::new(),
            error: None,
            highlight: None,
            view_mode: ViewMode::Details,
            remote_session: Some(session),
        }
    }
}

/// Application principale.
pub struct ExplorerApp {
    /// Onglets ouverts : chaque onglet isole un système (local ou distant).
    pub tabs: Vec<TabState>,
    /// Index de l'onglet actif.
    pub active_tab: usize,
    /// Contenu du presse-papiers interne (copier / couper).
    pub clipboard: Option<Clipboard>,
    /// Dialogue actif (modaux globaux).
    pub dialog: Dialog,
    /// Notification animée en cours d'affichage (icône + message).
    pub toast: Option<Toast>,
    /// Cache d'icônes système et de vignettes (textures egui).
    pub icons: crate::icons::IconCache,
    /// Mot de passe maître déverrouillé en mémoire (None = verrouillé).
    pub master_password: Option<String>,
    /// Magasin d'identifiants chiffrés (Some une fois déverrouillé).
    pub credentials: Option<credentials::CredentialsStore>,
    /// Une opération distante est-elle en cours ?
    pub remote_connecting: bool,
    /// Récepteur du résultat de l'opération distante en cours.
    pub remote_rx: Option<mpsc::Receiver<Result<RemoteMsg, String>>>,
    /// Type d'opération distante en cours (pour router le résultat).
    pub remote_awaiting: RemoteAwaiting,
    /// Dernière erreur de connexion distante.
    pub remote_error: Option<String>,
    /// État éphémère de la boîte de dialogue de connexion distante.
    pub remote_dialog: RemoteDialogState,
    /// État éphémère du déverrouillage du mot de passe maître.
    pub unlock_dialog: UnlockDialogState,

    /// Éléments supprimés en cours de disparition (animation de sortie macOS).
    pub vanishing: Vec<(PathBuf, Instant)>,
    /// Éléments ajoutés en cours d'apparition (animation d'entrée macOS).
    pub added: Vec<(PathBuf, Instant)>,
    /// Direction et instant de la dernière navigation (animation du contenu).
    pub nav_dir: Option<(NavDir, Instant)>,
}

impl ExplorerApp {
    /// Accède à l'onglet actif (navigation isolée).
    pub fn tab(&self) -> &TabState {
        &self.tabs[self.active_tab]
    }

    /// Accède mutablement à l'onglet actif.
    pub fn tab_mut(&mut self) -> &mut TabState {
        &mut self.tabs[self.active_tab]
    }

    /// Indique si l'onglet actif affiche un répertoire distant.
    pub fn is_remote(&self) -> bool {
        self.tab().remote_session.is_some()
    }

    /// Déverrouille le magasin d'identifiants avec le mot de passe maître.
    pub fn unlock_credentials(&mut self, password: String) -> Result<(), String> {
        let path = credentials::default_config_path();
        let store = credentials::CredentialsStore::open(&path, &password)
            .map_err(|e| format!("Impossible d'ouvrir le magasin : {e}"))?;
        self.master_password = Some(password);
        self.credentials = Some(store);
        Ok(())
    }

    /// Crée l'application avec un seul onglet local dans le répertoire home.
    pub fn new() -> Self {
        let start = fs::home_dir()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("/"));

        let mut app = ExplorerApp {
            tabs: vec![TabState::new_local(start.clone())],
            active_tab: 0,
            clipboard: None,
            dialog: Dialog::None,
            toast: None,
            icons: crate::icons::IconCache::new(),
            master_password: None,
            credentials: None,
            remote_connecting: false,
            remote_rx: None,
            remote_awaiting: RemoteAwaiting::Connect,
            remote_error: None,
            remote_dialog: RemoteDialogState::default(),
            unlock_dialog: UnlockDialogState::default(),
            vanishing: Vec::new(),
            added: Vec::new(),
            nav_dir: None,
        };
        app.expand_path();
        app.refresh();
        app
    }

    /// Déplie dans l'arborescence tous les parents du répertoire courant.
    fn expand_path(&mut self) {
        let cwd = self.tab().cwd.clone();
        let mut p = cwd.as_path();
        while let Some(parent) = p.parent() {
            self.tab_mut().expanded.insert(parent.to_path_buf());
            p = parent;
        }
    }

    /// Ouvre l'onglet `index` et recharge son contenu.
    pub fn switch_tab(&mut self, index: usize) {
        if index >= self.tabs.len() || index == self.active_tab {
            return;
        }
        self.active_tab = index;
        self.expand_path();
        self.refresh();
    }

    /// Ferme l'onglet `index`. Renvoie `false` s'il s'agit du dernier
    /// onglet (la fenêtre doit alors être fermée).
    pub fn close_tab(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() {
            return true;
        }
        if self.tabs.len() == 1 {
            return false;
        }
        // Ferme proprement la session distante de l'onglet fermé, le cas échéant.
        if let Some(mut session) = self
            .tabs
            .get_mut(index)
            .and_then(|t| t.remote_session.take())
        {
            std::thread::spawn(move || {
                let _ = session.fs.disconnect();
            });
        }
        self.tabs.remove(index);
        if index < self.active_tab {
            self.active_tab -= 1;
        } else {
            self.active_tab = self.active_tab.min(self.tabs.len() - 1);
        }
        self.expand_path();
        self.refresh();
        true
    }

    /// Ouvre un nouvel onglet local dans le répertoire personnel.
    pub fn new_tab(&mut self) {
        let home = fs::home_dir().unwrap_or_else(|| self.tab().cwd.clone());
        self.tabs.push(TabState::new_local(home));
        self.active_tab = self.tabs.len() - 1;
        self.expand_path();
        self.refresh();
    }

    /// Affiche une notification animée (icône + message) en bas à droite.
    /// Une nouvelle notification remplace la précédente.
    pub fn notify(&mut self, icon: &str, message: impl Into<String>) {
        self.toast = Some(Toast {
            icon: icon.to_string(),
            message: message.into(),
            created_at: Instant::now(),
            lifetime: 3.2,
        });
    }

    /// Démarre le flash animé de surlignage sur un élément de la liste.
    pub fn set_highlight(&mut self, path: PathBuf) {
        self.tab_mut().highlight = Some((path, Instant::now()));
    }

    /// Retire les entrées d'animation dont la durée est dépassée.
    /// À appeler à chaque frame (depuis `ui::show`).
    pub fn tick_animations(&mut self) {
        let now = Instant::now();
        self.vanishing
            .retain(|(_, t)| now.duration_since(*t).as_secs_f32() < animation::DURATION);
        self.added
            .retain(|(_, t)| now.duration_since(*t).as_secs_f32() < animation::DURATION);
        // On oublie la direction de navigation une fois l'animation finie.
        if let Some((_, t)) = self.nav_dir {
            if now.duration_since(t).as_secs_f32() >= animation::DURATION {
                self.nav_dir = None;
            }
        }
    }

    /// Navigue vers un dossier (ajoute à l'historique).
    pub fn navigate(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        if !path.is_dir() {
            return;
        }
        // Ne remplace pas une direction de navigation déjà en cours d'animation.
        let already_animating = self.nav_dir.is_some_and(|(_, t)| {
            Instant::now().duration_since(t).as_secs_f32() < animation::DURATION
        });
        if !already_animating {
            self.nav_dir = Some((NavDir::In, Instant::now()));
        }
        let tab = self.tab_mut();
        // Coupe l'historique "avant" si on avait navigué en arrière.
        tab.history.truncate(tab.history_index + 1);
        if tab.history.last() != Some(&path) {
            tab.history.push(path.clone());
            tab.history_index = tab.history.len() - 1;
        }
        tab.cwd = path;
        self.refresh();
    }

    /// Navigation "retour".
    pub fn go_back(&mut self) {
        self.nav_dir = Some((NavDir::Back, Instant::now()));
        if self.tab().history_index > 0 {
            let tab = self.tab_mut();
            tab.history_index -= 1;
            tab.cwd = tab.history[tab.history_index].clone();
            self.refresh();
        }
    }

    /// Navigation "avancer".
    pub fn go_forward(&mut self) {
        self.nav_dir = Some((NavDir::Forward, Instant::now()));
        let tab = self.tab();
        if tab.history_index + 1 < tab.history.len() {
            let tab = self.tab_mut();
            tab.history_index += 1;
            tab.cwd = tab.history[tab.history_index].clone();
            self.refresh();
        }
    }

    /// Remonte d'un niveau.
    pub fn go_up(&mut self) {
        if let Some(parent) = self.tab().cwd.parent() {
            let parent = parent.to_path_buf();
            self.nav_dir = Some((NavDir::Out, Instant::now()));
            self.navigate(parent);
        }
    }

    pub fn can_back(&self) -> bool {
        self.tab().history_index > 0
    }

    pub fn can_forward(&self) -> bool {
        let tab = self.tab();
        tab.history_index + 1 < tab.history.len()
    }

    /// Recharge le contenu du répertoire courant et applique tri + filtre.
    pub fn refresh(&mut self) {
        self.tab_mut().selected.clear();
        self.tab_mut().renaming = None;

        // Navigation distante : le listing est lancé de façon asynchrone sur
        // un worker thread (opération bloquante). Le résultat est appliqué
        // via `process_remote_messages` aux prochaines frames egui.
        if self.tab().remote_session.is_some() {
            self.refresh_remote();
            return;
        }

        match fs::list_dir(&self.tab().cwd, self.tab().show_hidden) {
            Ok(mut entries) => {
                // Recherche récursive si activée et texte non vide.
                if self.tab().search_recursive && !self.tab().search.trim().is_empty() {
                    let results = Self::search_recursive(
                        &self.tab().cwd,
                        &self.tab().search,
                        self.tab().show_hidden,
                    );
                    self.tab_mut().search_results = results.clone();
                    self.tab_mut().entries = results;
                } else {
                    // Filtre par nom.
                    let needle = self.tab().search.to_lowercase();
                    if !needle.is_empty() {
                        entries.retain(|e| e.name.to_lowercase().contains(&needle));
                    }
                    self.tab_mut().entries = entries;
                }
                self.sort_entries();
                self.update_status();
                self.tab_mut().error = None;
            }
            Err(e) => {
                self.tab_mut().entries.clear();
                self.tab_mut().error = Some(format!(
                    "Impossible d'ouvrir « {} » : {e}",
                    self.tab().cwd.display()
                ));
                self.update_status();
            }
        }
    }

    /// Convertit une `RemoteEntry` en `fs::Entry` réutilisable par la vue
    /// partagée (chemin synthétique, non utilisable pour des E/S locales).
    fn remote_entry_to_entry(re: &remote::RemoteEntry, base: &str) -> fs::Entry {
        let path = PathBuf::from(format!("{}/{}", base.trim_end_matches('/'), re.name));
        let extension = re.name.rsplit('.').next().unwrap_or("").to_string();
        fs::Entry {
            path,
            name: re.name.clone(),
            is_dir: re.is_dir,
            is_hidden: false,
            size: re.size,
            modified: re.modified,
            extension,
            is_symlink: false,
        }
    }

    /// Lance la connexion à un serveur distant sur un worker thread.
    /// Si `save` est vrai et que le magasin est déverrouillé, le serveur est
    /// enregistré (chiffré) pour les prochaines sessions.
    pub fn connect_remote(&mut self, cfg: remote::RemoteConfig, save: bool, ctx: &egui::Context) {
        if save {
            let Some(store) = self.credentials.as_mut() else {
                self.remote_error = Some(
                    "Mot de passe maître non déverrouillé : impossible d'enregistrer.".to_string(),
                );
                return;
            };
            let cfg_to_save = cfg.clone();
            if let Err(e) = store
                .add_or_update_server(cfg_to_save, self.master_password.as_deref().unwrap_or(""))
            {
                self.remote_error = Some(format!("Impossible d'enregistrer le serveur : {e}"));
                // On tente quand même la connexion.
            }
        }

        self.remote_connecting = true;
        self.remote_awaiting = RemoteAwaiting::Connect;
        self.remote_error = None;

        let (tx, rx) = mpsc::channel();
        self.remote_rx = Some(rx);

        let cfg_for_thread = cfg.clone();
        std::thread::spawn(move || {
            let mut fs = remote::make_remote_fs(cfg.protocol);
            let result = fs.connect(&cfg);
            let session = match result {
                Ok(()) => {
                    // On liste la racine immédiatement pour éviter une
                    // seconde connexion au premier rafraîchissement.
                    let entries = fs.list_dir("/").unwrap_or_default();
                    remote::RemoteSession {
                        config: cfg_for_thread,
                        fs,
                        cwd: "/".to_string(),
                        entries,
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("Connexion échouée : {e}")));
                    return;
                }
            };
            let _ = tx.send(Ok(RemoteMsg::Connected(session)));
        });

        ctx.request_repaint();
    }

    /// Déconnecte la session distante de l'onglet actif (fermeture bloquante
    /// sur un worker). Quand l'onglet distant est le seul, on le réinitialise
    /// en local ; sinon on ferme l'onglet distant.
    pub fn disconnect_remote(&mut self) {
        let session = self.tab_mut().remote_session.take();
        if let Some(mut session) = session {
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = session.fs.disconnect();
                let _ = tx.send(());
            });
            let _ = rx.recv_timeout(std::time::Duration::from_secs(5));
        }

        if self.tabs.len() > 1 {
            let _ = self.close_tab(self.active_tab);
        } else {
            // Seul onglet : on le réinitialise en local (dossier home).
            let home = fs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            *self.tab_mut() = TabState::new_local(home);
            self.refresh();
        }
        self.remote_error = None;
    }

    /// Lance le listage du répertoire courant de la session distante de
    /// l'onglet actif.
    fn refresh_remote(&mut self) {
        let config = match self.tab().remote_config.clone() {
            Some(cfg) => cfg,
            None => return,
        };
        let cwd = self.tab().cwd.to_string_lossy().to_string();

        self.remote_connecting = true;
        self.remote_awaiting = RemoteAwaiting::ListDir;
        self.remote_error = None;

        let (tx, rx) = mpsc::channel();
        self.remote_rx = Some(rx);

        std::thread::spawn(move || {
            let mut fs = remote::make_remote_fs(config.protocol);
            let result = (|| {
                fs.connect(&config)?;
                fs.list_dir(&cwd)
            })();
            let _ = match result {
                Ok(entries) => tx.send(Ok(RemoteMsg::Listed(entries))),
                Err(e) => tx.send(Err(format!("Listage échoué : {e}"))),
            };
        });
    }

    /// Change le répertoire courant de la session distante de l'onglet actif.
    pub fn navigate_remote(&mut self, path: &str) {
        if self.tab().remote_session.is_some() {
            self.tab_mut().cwd = PathBuf::from(path);
            self.refresh_remote();
        }
    }

    /// Lit le contenu d'un fichier distant sur un worker thread.
    pub fn read_remote_file(&mut self, path: String, ctx: &egui::Context) {
        let config = match self.tab().remote_config.clone() {
            Some(cfg) => cfg,
            None => return,
        };

        self.remote_connecting = true;
        self.remote_awaiting = RemoteAwaiting::ReadFile;
        self.remote_error = None;

        let (tx, rx) = mpsc::channel();
        self.remote_rx = Some(rx);

        std::thread::spawn(move || {
            let mut fs = remote::make_remote_fs(config.protocol);
            let result = (|| {
                fs.connect(&config)?;
                fs.read_file(&path)
            })();
            let _ = match result {
                Ok(bytes) => tx.send(Ok(RemoteMsg::FileRead(bytes))),
                Err(e) => tx.send(Err(format!("Lecture échouée : {e}"))),
            };
        });

        ctx.request_repaint();
    }

    /// Traite les résultats d'opérations distantes arrivés sur le canal.
    /// À appeler à chaque frame egui (depuis `ui::show`).
    pub fn process_remote_messages(&mut self, ctx: &egui::Context) {
        if !self.remote_connecting {
            return;
        }
        let Some(rx) = self.remote_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(msg) => {
                self.remote_connecting = false;
                match msg {
                    Ok(RemoteMsg::Connected(session)) => {
                        // Ouvre la connexion dans un nouvel onglet distant
                        // isolé, puis l'active.
                        self.tabs
                            .push(TabState::new_remote(session.config.clone(), session));
                        self.active_tab = self.tabs.len() - 1;
                        self.sort_entries();
                        self.update_status();
                        self.remote_error = None;
                    }
                    Ok(RemoteMsg::Listed(entries)) => {
                        let base = self
                            .tab()
                            .remote_session
                            .as_ref()
                            .map(|s| s.cwd.clone())
                            .unwrap_or_else(|| "/".to_string());
                        self.tab_mut().entries = entries
                            .into_iter()
                            .map(|re| Self::remote_entry_to_entry(&re, &base))
                            .collect();
                        self.sort_entries();
                        self.update_status();
                        self.tab_mut().error = None;
                    }
                    Ok(RemoteMsg::FileRead(bytes)) => {
                        // TODO: afficher le contenu du fichier dans un panneau dédié.
                        let _ = bytes;
                    }
                    Err(e) => self.remote_error = Some(e),
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.remote_rx = Some(rx);
                ctx.request_repaint();
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.remote_connecting = false;
                self.remote_error = Some("Connexion distante interrompue.".to_string());
            }
        }
    }

    /// Recherche récursive dans les sous-dossiers (limité en profondeur).
    fn search_recursive(root: &std::path::Path, needle: &str, show_hidden: bool) -> Vec<fs::Entry> {
        let mut results = Vec::new();
        let needle = needle.to_lowercase();
        let mut stack: Vec<std::path::PathBuf> = vec![root.to_path_buf()];
        let mut visited = 0;
        while let Some(dir) = stack.pop() {
            visited += 1;
            if visited > 20_000 {
                break; // garde-fou
            }
            if let Ok(entries) = fs::list_dir(&dir, show_hidden) {
                for e in entries {
                    if e.name.to_lowercase().contains(&needle) {
                        results.push(e.clone());
                    }
                    if e.is_dir {
                        stack.push(e.path);
                    }
                }
            }
        }
        results
    }

    /// Applique le tri courant (les dossiers toujours en premier).
    pub fn sort_entries(&mut self) {
        let col = self.tab().sort_column;
        let asc = self.tab().sort_order == SortOrder::Asc;
        self.tab_mut().entries.sort_by(|a, b| {
            // Les dossiers avant les fichiers.
            match (a.is_dir, b.is_dir) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            let ord = match col {
                SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortColumn::Size => a.size.cmp(&b.size),
                SortColumn::Modified => a.modified.cmp(&b.modified),
                SortColumn::Type => a.extension.cmp(&b.extension),
            };
            if asc { ord } else { ord.reverse() }
        });
    }

    /// Change de colonne de tri (inverse le sens si même colonne).
    pub fn toggle_sort(&mut self, col: SortColumn) {
        let tab = self.tab_mut();
        if tab.sort_column == col {
            tab.sort_order = match tab.sort_order {
                SortOrder::Asc => SortOrder::Desc,
                SortOrder::Desc => SortOrder::Asc,
            };
        } else {
            tab.sort_column = col;
            tab.sort_order = SortOrder::Asc;
        }
        self.sort_entries();
    }

    /// Met à jour le texte de la barre d'état (nb d'éléments, sélection).
    pub fn update_status(&mut self) {
        let total = self.tab().entries.len();
        let sel = self.tab().selected.len();
        if sel > 0 {
            let total_size: u64 = self
                .tab()
                .entries
                .iter()
                .filter(|e| self.tab().selected.contains(&e.path))
                .map(|e| e.size)
                .sum();
            self.tab_mut().status = format!(
                "{sel} élément(s) sur {total} — {}",
                fs::format_size(total_size)
            );
        } else {
            self.tab_mut().status = format!("{total} élément(s)");
        }
    }

    /// Gère le clic sur une ligne (avec Ctrl/Shift pour multi-sélection).
    /// `index` est l'index de l'élément dans les entrées de l'onglet actif.
    pub fn handle_click(&mut self, index: usize, ctrl: bool, shift: bool) {
        let Some(clicked) = self.tab().entries.get(index) else {
            return;
        };
        let clicked_path = clicked.path.clone();
        let tab = self.tab_mut();

        if ctrl {
            if tab.selected.contains(&clicked_path) {
                tab.selected.remove(&clicked_path);
            } else {
                tab.selected.insert(clicked_path.clone());
                tab.selection_anchor = Some(clicked_path);
            }
        } else if shift {
            if let Some(anchor_path) = tab.selection_anchor.clone() {
                if let (Some(a_idx), Some(b_idx)) = (
                    tab.entries.iter().position(|e| e.path == anchor_path),
                    Some(index),
                ) {
                    let (lo, hi) = if a_idx <= b_idx {
                        (a_idx, b_idx)
                    } else {
                        (b_idx, a_idx)
                    };
                    tab.selected.clear();
                    for e in &tab.entries[lo..=hi] {
                        tab.selected.insert(e.path.clone());
                    }
                }
            } else {
                tab.selected.clear();
                tab.selected.insert(clicked_path.clone());
                tab.selection_anchor = Some(clicked_path);
            }
        } else {
            tab.selected.clear();
            tab.selected.insert(clicked_path.clone());
            tab.selection_anchor = Some(clicked_path);
        }
        self.update_status();
    }

    /// Sélectionne tout.
    pub fn select_all(&mut self) {
        let tab = self.tab_mut();
        tab.selected = tab.entries.iter().map(|e| e.path.clone()).collect();
        self.update_status();
    }

    /// Lance le renommage de l'élément sélectionné (boîte de dialogue).
    /// Si un seul élément est sélectionné, ouvre le dialogue de renommage.
    pub fn rename_selected(&mut self) {
        let tab = self.tab();
        let paths: Vec<PathBuf> = tab.selected.iter().cloned().collect();
        if paths.len() == 1 {
            let path = paths[0].clone();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            self.dialog = Dialog::Rename { path, name };
        }
    }

    /// Ouvre un élément : navigue si dossier, ouvre avec l'app par défaut sinon.
    pub fn open_entry(&mut self, ctx: &egui::Context, path: &std::path::Path) {
        if self.is_remote() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let is_dir = self
                .tab()
                .remote_session
                .as_ref()
                .and_then(|s| s.entries.iter().find(|e| e.name == name))
                .map(|e| e.is_dir)
                .unwrap_or(false);
            let remote_path = path.to_string_lossy().to_string();
            if is_dir {
                self.navigate_remote(&remote_path);
            } else {
                self.read_remote_file(remote_path, ctx);
            }
            return;
        }
        if path.is_dir() {
            self.navigate(path.to_path_buf());
        } else if let Err(e) = fs::open_with_default(path) {
            self.tab_mut().error = Some(format!("Ouverture impossible : {e}"));
        }
    }

    /// Crée un dossier dans le répertoire courant de l'onglet actif.
    /// Retourne `true` si la création a réussi (le dialogue peut alors se fermer).
    pub fn create_new_folder(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() {
            self.tab_mut().error = Some("Le nom ne peut pas être vide.".to_string());
            return false;
        }
        let tab = self.tab_mut();
        let path = tab.cwd.join(name);
        if let Err(e) = fs::create_dir(&path) {
            tab.error = Some(format!("Création impossible : {e}"));
            false
        } else {
            self.refresh();
            self.added.push((path.clone(), Instant::now()));
            self.set_highlight(path);
            self.notify("📁", format!("Dossier « {name} » créé"));
            true
        }
    }

    /// Crée un fichier vide dans le répertoire courant de l'onglet actif.
    /// Retourne `true` si la création a réussi (le dialogue peut alors se fermer).
    pub fn create_new_file(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() {
            self.tab_mut().error = Some("Le nom ne peut pas être vide.".to_string());
            return false;
        }
        let tab = self.tab_mut();
        let path = tab.cwd.join(name);
        if let Err(e) = fs::create_file(&path) {
            tab.error = Some(format!("Création impossible : {e}"));
            false
        } else {
            self.refresh();
            self.added.push((path.clone(), Instant::now()));
            self.set_highlight(path);
            self.notify("📄", format!("Fichier « {name} » créé"));
            true
        }
    }

    /// Supprime une liste d'éléments (vers la corbeille, avec repli définitif).
    pub fn delete_paths(&mut self, paths: &[PathBuf]) {
        let mut errors = Vec::new();
        for p in paths {
            if let Err(e) = fs::move_to_trash(p) {
                // Repli : suppression définitive.
                if fs::remove_permanently(p).is_err() {
                    errors.push(format!("{} : {e}", p.display()));
                }
            }
        }
        for p in paths {
            self.vanishing.push((p.clone(), Instant::now()));
        }
        self.tab_mut().selected.clear();
        self.refresh();
        if !errors.is_empty() {
            self.tab_mut().error =
                Some(format!("Erreur(s) de suppression : {}", errors.join("; ")));
        } else if !paths.is_empty() {
            self.tab_mut().status =
                format!("{} élément(s) déplacé(s) vers la corbeille", paths.len());
            self.notify(
                "🗑️",
                format!("{} élément(s) déplacé(s) vers la corbeille", paths.len()),
            );
        }
    }

    /// Copie la sélection dans le presse-papiers.
    pub fn copy_selection(&mut self, cut: bool) {
        let tab = self.tab();
        let paths: Vec<PathBuf> = tab.selected.iter().cloned().collect();
        if !paths.is_empty() {
            let count = paths.len();
            self.clipboard = Some(Clipboard { paths, cut });
            self.tab_mut().status = if cut {
                "Éléments coupés. Collez pour déplacer.".to_string()
            } else {
                "Éléments copiés.".to_string()
            };
            if cut {
                self.notify("✂️", format!("{count} élément(s) coupé(s)"));
            } else {
                self.notify("📋", format!("{count} élément(s) copié(s)"));
            }
        }
    }

    /// Colle le presse-papiers dans un dossier destination.
    pub fn paste_in(&mut self, dest: &std::path::Path) {
        let clipboard = match self.clipboard.clone() {
            Some(c) => c,
            None => {
                self.tab_mut().status = "Presse-papiers vide.".to_string();
                return;
            }
        };
        let dest = dest.to_path_buf();
        let mut errors = Vec::new();
        let mut last_created: Option<PathBuf> = None;
        let mut ok_count = 0usize;
        for src in &clipboard.paths {
            let name = match src.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };
            let dest_path = fs::unique_dest(&dest, &name);
            let result = if clipboard.cut {
                fs::move_path(src, &dest_path)
            } else {
                fs::copy_path(src, &dest_path)
            };
            if let Err(e) = result {
                errors.push(format!("{} : {e}", src.display()));
            } else {
                last_created = Some(dest_path);
                ok_count += 1;
            }
        }
        if clipboard.cut {
            self.clipboard = None;
        }
        self.refresh();
        if let Some(p) = last_created {
            self.added.push((p.clone(), Instant::now()));
            self.set_highlight(p);
        }
        if !errors.is_empty() {
            self.tab_mut().error = Some(format!("Erreur(s) de collage : {}", errors.join("; ")));
        } else {
            self.tab_mut().status = "Collage effectué.".to_string();
            if ok_count > 0 {
                self.notify("📥", format!("{ok_count} élément(s) collé(s)"));
            }
        }
    }

    /// Colle dans le répertoire courant.
    pub fn paste(&mut self) {
        let cwd = self.tab().cwd.clone();
        self.paste_in(&cwd);
    }

    /// Renomme un élément. Retourne `true` si le renommage a réussi
    /// (le dialogue peut alors se fermer).
    pub fn rename_entry(&mut self, path: &PathBuf, new_name: &str) -> bool {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            self.tab_mut().error = Some("Nom vide.".to_string());
            return false;
        }
        let Some(parent) = path.parent() else {
            self.tab_mut().error = Some("Chemin invalide.".to_string());
            return false;
        };
        let new_path = parent.join(new_name);
        if new_path == *path {
            // Nom inchangé : on ferme simplement la boîte de dialogue.
            self.tab_mut().renaming = None;
            return true;
        }
        if new_path.exists() {
            self.tab_mut().error = Some(format!("« {new_name} » existe déjà."));
            return false;
        }
        if let Err(e) = fs::rename(path, &new_path) {
            self.tab_mut().error = Some(format!("Renommage impossible : {e}"));
            return false;
        }
        self.tab_mut().renaming = None;
        self.refresh();
        self.set_highlight(new_path);
        self.notify("✏️", format!("Renommé en « {new_name} »"));
        true
    }
}
