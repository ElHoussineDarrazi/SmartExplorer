//! Stockage chiffré des identifiants des serveurs distants.
//!
//! Les identifiants (host, port, utilisateur, mot de passe) sont sérialisés en
//! JSON, puis chiffrés avec XChaCha20-Poly1305. La clé de chiffrement est
//! dérivée d'un mot de passe maître fourni par l'utilisateur via Argon2id.
//! Le fichier résultant est écrit dans le répertoire de configuration de l'OS.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    Key, KeyInit, XChaCha20Poly1305,
    aead::{Aead, Nonce},
};
use thiserror::Error;

use crate::remote::RemoteConfig;

const CONFIG_FILE_NAME: &str = "servers.enc";
const APP_NAME: &str = "SmartExplorer";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24; // XChaCha20-Poly1305 utilise un nonce de 192 bits
const KEY_LEN: usize = 32;

/// Erreurs du magasin d'identifiants.
#[derive(Debug, Error)]
pub enum CredentialsError {
    #[error("I/O: {0}")]
    Io(#[from] io::Error),
    #[error("chiffrement: {0}")]
    Crypto(String),
    #[error("sérialisation: {0}")]
    Serialization(String),
    #[error("mot de passe maître incorrect (données corrompues ou mauvais mot de passe)")]
    WrongPassword,
}

/// Magasin d'identifiants déchiffrés en mémoire.
pub struct CredentialsStore {
    /// Liste des serveurs distants configurés (mots de passe déchiffrés).
    pub servers: Vec<RemoteConfig>,
    /// Chemin du fichier de stockage chiffré.
    path: PathBuf,
}

impl CredentialsStore {
    /// Chemin par défaut du fichier de stockage, basé sur le répertoire de
    /// configuration de l'OS.
    pub fn default_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(APP_NAME);
        fs::create_dir_all(&path).ok();
        path.push(CONFIG_FILE_NAME);
        path
    }

    /// Ouvre (ou crée) le magasin à partir du fichier `path` en utilisant
    /// `master_password`. Si le fichier n'existe pas, un magasin vide est
    /// créé et sera écrit à la première modification.
    pub fn open(path: &Path, master_password: &str) -> Result<Self, CredentialsError> {
        let data = match fs::read(path) {
            Ok(data) => data,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(CredentialsStore {
                    servers: Vec::new(),
                    path: path.to_path_buf(),
                });
            }
            Err(e) => return Err(CredentialsError::Io(e)),
        };

        if data.len() < SALT_LEN + NONCE_LEN {
            return Err(CredentialsError::WrongPassword);
        }

        let (salt, rest) = data.split_at(SALT_LEN);
        let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);

        let key = derive_key(master_password, salt)?;
        let key: Key = key
            .as_slice()
            .try_into()
            .map_err(|_| CredentialsError::Crypto("invalid key length".to_string()))?;
        let cipher = XChaCha20Poly1305::new(&key);
        let nonce_arr: [u8; 24] = nonce_bytes
            .try_into()
            .map_err(|_| CredentialsError::Crypto("invalid nonce length".to_string()))?;
        let nonce: &Nonce<XChaCha20Poly1305> = (&nonce_arr).into();

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CredentialsError::WrongPassword)?;

        let servers: Vec<RemoteConfig> = serde_json::from_slice(&plaintext)
            .map_err(|e| CredentialsError::Serialization(e.to_string()))?;

        Ok(CredentialsStore {
            servers,
            path: path.to_path_buf(),
        })
    }

    /// Ajoute ou met à jour un serveur, puis persiste le magasin chiffré.
    pub fn add_or_update_server(
        &mut self,
        cfg: RemoteConfig,
        master_password: &str,
    ) -> Result<(), CredentialsError> {
        if let Some(pos) = self.servers.iter().position(|s| s.id == cfg.id) {
            self.servers[pos] = cfg;
        } else {
            self.servers.push(cfg);
        }
        self.save(master_password)
    }

    /// Chiffre et écrit le magasin sur disque.
    fn save(&self, master_password: &str) -> Result<(), CredentialsError> {
        let salt = random_bytes(SALT_LEN);
        let key = derive_key(master_password, &salt)?;
        let key: Key = key
            .as_slice()
            .try_into()
            .map_err(|_| CredentialsError::Crypto("invalid key length".to_string()))?;
        let cipher = XChaCha20Poly1305::new(&key);

        let plaintext = serde_json::to_vec(&self.servers)
            .map_err(|e| CredentialsError::Serialization(e.to_string()))?;

        let nonce_bytes = random_bytes(NONCE_LEN);
        let nonce_arr: [u8; 24] = nonce_bytes
            .as_slice()
            .try_into()
            .map_err(|_| CredentialsError::Crypto("invalid nonce length".to_string()))?;
        let nonce: &Nonce<XChaCha20Poly1305> = (&nonce_arr).into();

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_slice())
            .map_err(|e| CredentialsError::Crypto(e.to_string()))?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).ok();
        }

        let mut out = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&salt);
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);

        fs::write(&self.path, out).map_err(CredentialsError::Io)?;
        Ok(())
    }
}

/// Dérive une clé de 32 octets à partir du mot de passe maître et d'un sel
/// via Argon2id.
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], CredentialsError> {
    let params =
        Params::new(65536, 3, 1, None).map_err(|e| CredentialsError::Crypto(e.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; KEY_LEN];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| CredentialsError::Crypto(e.to_string()))?;
    Ok(key)
}

/// Génère `n` octets aléatoires de manière cryptographiquement sûre.
fn random_bytes(n: usize) -> Vec<u8> {
    use rand::RngCore;
    let mut buf = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

/// Chemin par défaut du fichier de stockage (ré-exporté pour l'UI).
pub fn default_config_path() -> PathBuf {
    CredentialsStore::default_path()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_save_and_load() {
        let dir = std::env::temp_dir().join(format!("smart_explorer_test_{}", std::process::id()));
        fs::create_dir_all(&dir).ok();
        let path = dir.join("servers.enc");
        let _ = fs::remove_file(&path);

        let password = "master-test-12345";
        let mut store = CredentialsStore::open(&path, password).unwrap();
        store
            .add_or_update_server(
                RemoteConfig {
                    id: "srv-1".to_string(),
                    name: "Mon serveur".to_string(),
                    protocol: crate::remote::RemoteProtocol::Ssh,
                    host: "192.168.1.50".to_string(),
                    port: 22,
                    user: "alice".to_string(),
                    password: Some("super-secret".to_string()),
                },
                password,
            )
            .unwrap();

        // Rechargement avec le même mot de passe.
        let loaded = CredentialsStore::open(&path, password).unwrap();
        assert_eq!(loaded.servers.len(), 1);
        assert_eq!(loaded.servers[0].id, "srv-1");
        assert_eq!(loaded.servers[0].password.as_deref(), Some("super-secret"));

        // Mauvais mot de passe -> erreur.
        assert!(CredentialsStore::open(&path, "mauvais").is_err());

        let _ = fs::remove_dir_all(&dir);
    }
}
