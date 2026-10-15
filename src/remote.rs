//! Abstraction des systèmes de fichiers distants (SSH/SFTP et FTP).
//!
//! Toutes les opérations réseau sont bloquantes : elles doivent être appelées
//! depuis un worker thread (`std::thread::spawn`) pour ne pas geler la boucle
//! immédiate d'egui. Le trait `RemoteFileSystem` est donc `Send + Sync`.

use std::io::Read;
use std::net::TcpStream;
use std::path::Path;

use ssh2::Session;

use chrono::{DateTime, Local};
use thiserror::Error;

/// Protocole de connexion supporté.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RemoteProtocol {
    #[default]
    Ssh,
    Ftp,
}

impl std::fmt::Display for RemoteProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteProtocol::Ssh => write!(f, "SSH / SFTP"),
            RemoteProtocol::Ftp => write!(f, "FTP"),
        }
    }
}

/// Entrée d'un répertoire distant (cohérence avec `fs::Entry`).
///
/// Pas de `serde` ici : on ne sérialise que la configuration (`RemoteConfig`),
/// jamais le listing d'un répertoire.
#[derive(Clone, Debug)]
pub struct RemoteEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<DateTime<Local>>,
}

/// Configuration d'un serveur distant (sérialisée dans le fichier de config).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RemoteConfig {
    pub id: String,
    pub name: String,
    pub protocol: RemoteProtocol,
    pub host: String,
    pub port: u16,
    pub user: String,
    /// Mot de passe (chiffré par `CredentialsStore` lorsqu'il est persistant).
    pub password: Option<String>,
}

impl RemoteConfig {
    pub fn display_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Erreurs liées aux opérations distantes.
#[derive(Debug, Error)]
pub enum RemoteError {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("SSH: {0}")]
    Ssh(String),
    #[error("FTP: {0}")]
    Ftp(String),
    #[error("non connecté")]
    NotConnected,
    #[error("authentification échouée")]
    Auth,
    #[error("{0}")]
    Other(String),
}

impl From<String> for RemoteError {
    fn from(s: String) -> Self {
        RemoteError::Other(s)
    }
}

/// Trait d'abstraction sur un système de fichiers distant.
pub trait RemoteFileSystem: Send + Sync {
    /// Établit la connexion (bloquant).
    fn connect(&mut self, cfg: &RemoteConfig) -> Result<(), RemoteError>;
    /// Liste le contenu du répertoire `path` (bloquant).
    fn list_dir(&mut self, path: &str) -> Result<Vec<RemoteEntry>, RemoteError>;
    /// Lit le contenu complet d'un fichier (bloquant).
    fn read_file(&mut self, path: &str) -> Result<Vec<u8>, RemoteError>;
    /// Ferme la connexion (bloquant).
    fn disconnect(&mut self) -> Result<(), RemoteError>;
}
// ---------------------------------------------------------------------------
// SSH / SFTP (implémentation via `ssh2-rs` lié à libssh2, opérations bloquantes)
// ---------------------------------------------------------------------------

/// Client SSH/SFTP.
pub struct SshFs {
    session: Option<Session>,
}

impl SshFs {
    pub fn new() -> Self {
        SshFs { session: None }
    }
}

impl Default for SshFs {
    fn default() -> Self {
        Self::new()
    }
}

impl RemoteFileSystem for SshFs {
    fn connect(&mut self, cfg: &RemoteConfig) -> Result<(), RemoteError> {
        let tcp = TcpStream::connect((cfg.host.as_str(), cfg.port)).map_err(RemoteError::Io)?;
        let mut sess = Session::new().map_err(|e| RemoteError::Ssh(e.to_string()))?;
        sess.set_tcp_stream(tcp);
        sess.handshake()
            .map_err(|e| RemoteError::Ssh(e.to_string()))?;
        sess.userauth_password(&cfg.user, &cfg.password.clone().unwrap_or_default())
            .map_err(|e| RemoteError::Ssh(e.to_string()))?;
        if !sess.authenticated() {
            return Err(RemoteError::Auth);
        }
        self.session = Some(sess);
        Ok(())
    }

    fn list_dir(&mut self, path: &str) -> Result<Vec<RemoteEntry>, RemoteError> {
        let sess = self.session.as_ref().ok_or(RemoteError::NotConnected)?;
        let sftp = sess.sftp().map_err(|e| RemoteError::Ssh(e.to_string()))?;
        let stats = sftp
            .readdir(Path::new(path))
            .map_err(|e| RemoteError::Ssh(e.to_string()))?;
        let mut out = Vec::new();
        for (path, stat) in stats {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name == "." || name == ".." {
                continue;
            }
            out.push(RemoteEntry {
                name,
                is_dir: stat.is_dir(),
                size: stat.size.unwrap_or(0),
                modified: stat.mtime.and_then(|ts| {
                    DateTime::from_timestamp(ts as i64, 0).map(|dt| dt.with_timezone(&Local))
                }),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn read_file(&mut self, path: &str) -> Result<Vec<u8>, RemoteError> {
        let sess = self.session.as_ref().ok_or(RemoteError::NotConnected)?;
        let sftp = sess.sftp().map_err(|e| RemoteError::Ssh(e.to_string()))?;
        let mut file = sftp
            .open(Path::new(path))
            .map_err(|e| RemoteError::Ssh(e.to_string()))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| RemoteError::Ssh(e.to_string()))?;
        Ok(buf)
    }

    fn disconnect(&mut self) -> Result<(), RemoteError> {
        if let Some(sess) = self.session.take() {
            sess.disconnect(None, "", None)
                .map_err(|e| RemoteError::Ssh(e.to_string()))?;
        }
        Ok(())
    }
}
// ---------------------------------------------------------------------------
// FTP (implémentation synchrone pure Rust via le crate `ftp`)
// ---------------------------------------------------------------------------

use ftp::FtpStream;

/// Client FTP.
pub struct FtpFs {
    stream: Option<FtpStream>,
    cwd: String,
}

impl FtpFs {
    pub fn new() -> Self {
        FtpFs {
            stream: None,
            cwd: "/".to_string(),
        }
    }
}

impl Default for FtpFs {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_ftp_list_line(line: &str) -> Option<RemoteEntry> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    // Format Unix LIST :
    // -rw-r--r-- 1 user group 1234 Jan 1 12:00 filename
    let mut parts = line.splitn(9, |c: char| c.is_whitespace());
    let perms = parts.next()?;
    let _links = parts.next()?;
    let _user = parts.next()?;
    let _group = parts.next()?;
    let size_str = parts.next()?;
    let _month = parts.next()?;
    let _day = parts.next()?;
    let rest = parts.next()?; // "HH:MM filename" ou "YYYY filename"

    let is_dir = perms.starts_with('d') || perms.starts_with('l');
    let size: u64 = size_str.parse().unwrap_or(0);

    // Le nom est le reste après la date/heure.
    let name = rest
        .rsplit_once(' ')
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| rest.to_string());
    let name = name.trim().to_string();
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }

    Some(RemoteEntry {
        name,
        is_dir,
        size,
        modified: None, // le parsing de date est complexe ; laissé à None pour le spike
    })
}

impl RemoteFileSystem for FtpFs {
    fn connect(&mut self, cfg: &RemoteConfig) -> Result<(), RemoteError> {
        let addr = (cfg.host.as_str(), cfg.port);
        let mut stream = FtpStream::connect(addr).map_err(|e| RemoteError::Ftp(e.to_string()))?;
        stream
            .login(&cfg.user, &cfg.password.clone().unwrap_or_default())
            .map_err(|e| RemoteError::Ftp(e.to_string()))?;
        let _ = stream.cwd("/");
        self.cwd = "/".to_string();
        self.stream = Some(stream);
        Ok(())
    }

    fn list_dir(&mut self, path: &str) -> Result<Vec<RemoteEntry>, RemoteError> {
        let stream = self.stream.as_mut().ok_or(RemoteError::NotConnected)?;
        let target = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("{}/{}", self.cwd.trim_end_matches('/'), path)
        };

        let lines: Vec<String> = stream
            .list(Some(&target))
            .map_err(|e| RemoteError::Ftp(e.to_string()))?;

        let mut entries = Vec::new();
        for line in lines {
            if let Some(entry) = parse_ftp_list_line(&line) {
                entries.push(entry);
            }
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn read_file(&mut self, path: &str) -> Result<Vec<u8>, RemoteError> {
        let stream = self.stream.as_mut().ok_or(RemoteError::NotConnected)?;
        let buf = stream
            .retr(path, |s| {
                let mut b = Vec::new();
                s.read_to_end(&mut b)
                    .map_err(ftp::FtpError::ConnectionError)?;
                Ok(b)
            })
            .map_err(|e| RemoteError::Ftp(e.to_string()))?;
        Ok(buf)
    }

    fn disconnect(&mut self) -> Result<(), RemoteError> {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.quit();
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Fabricante : crée le bon filesystem selon le protocole.
// ---------------------------------------------------------------------------

/// Crée une instance de filesystem distant selon le protocole de la config.
pub fn make_remote_fs(protocol: RemoteProtocol) -> Box<dyn RemoteFileSystem> {
    match protocol {
        RemoteProtocol::Ssh => Box::new(SshFs::new()),
        RemoteProtocol::Ftp => Box::new(FtpFs::new()),
    }
}

/// Session distante active : filesystem connecté + état de navigation.
pub struct RemoteSession {
    pub config: RemoteConfig,
    pub fs: Box<dyn RemoteFileSystem>,
    pub cwd: String,
    pub entries: Vec<RemoteEntry>,
}
