use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
};

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use keycard_rmcp::rmcp::transport::auth::{AuthError, CredentialStore, StoredCredentials};
use sha2::{Digest, Sha256};
use tokio::{io::AsyncWriteExt as _, sync::Mutex};

use crate::state::UserKey;

const STORAGE_DIR_ENV: &str = "GOOGLE_CALENDAR_OAUTH_STORAGE_DIR";
const ENCRYPTION_KEY_ENV: &str = "GOOGLE_CALENDAR_OAUTH_ENCRYPTION_KEY";
const FORMAT_VERSION: u8 = 1;
const NONCE_LEN: usize = 12;
const AAD_PREFIX: &str = "shortrib-agent/google-calendar/credentials/v1";

#[derive(Clone)]
pub(crate) struct CredentialVault {
    root: Arc<PathBuf>,
    key: Arc<[u8; 32]>,
}

impl CredentialVault {
    pub(crate) fn from_env() -> anyhow::Result<Option<Self>> {
        let root = std::env::var(STORAGE_DIR_ENV).ok();
        let encoded_key = std::env::var(ENCRYPTION_KEY_ENV).ok();
        match (root, encoded_key) {
            (None, None) => Ok(None),
            (Some(_), None) => anyhow::bail!(
                "{ENCRYPTION_KEY_ENV} is required when {STORAGE_DIR_ENV} is configured"
            ),
            (None, Some(_)) => anyhow::bail!(
                "{STORAGE_DIR_ENV} is required when {ENCRYPTION_KEY_ENV} is configured"
            ),
            (Some(root), Some(encoded_key)) => {
                let decoded = STANDARD
                    .decode(encoded_key)
                    .map_err(|_| anyhow::anyhow!("{ENCRYPTION_KEY_ENV} must be base64"))?;
                let key: [u8; 32] = decoded.try_into().map_err(|_| {
                    anyhow::anyhow!("{ENCRYPTION_KEY_ENV} must decode to exactly 32 bytes")
                })?;
                Ok(Some(Self {
                    root: Arc::new(PathBuf::from(root)),
                    key: Arc::new(key),
                }))
            }
        }
    }

    pub(crate) fn store(&self, user: &UserKey) -> EncryptedCredentialStore {
        EncryptedCredentialStore::new(
            self.root.as_ref().clone(),
            *self.key,
            user.oauth_storage_identity(),
        )
    }
}

#[derive(Clone)]
pub(crate) struct EncryptedCredentialStore {
    path: Arc<PathBuf>,
    key: Arc<[u8; 32]>,
    associated_data: Arc<Vec<u8>>,
    lock: Arc<Mutex<()>>,
}

impl EncryptedCredentialStore {
    fn new(root: PathBuf, key: [u8; 32], identity: String) -> Self {
        let digest = Sha256::digest(identity.as_bytes());
        let filename = format!("{}.credentials", hex(&digest));
        Self {
            path: Arc::new(root.join(filename)),
            key: Arc::new(key),
            associated_data: Arc::new(format!("{AAD_PREFIX}\0{identity}").into_bytes()),
            lock: Arc::new(Mutex::new(())),
        }
    }

    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, AuthError> {
        let cipher = Aes256Gcm::new_from_slice(self.key.as_ref())
            .map_err(|_| storage_error("invalid encryption key"))?;
        let mut nonce = [0_u8; NONCE_LEN];
        getrandom::fill(&mut nonce)
            .map_err(|_| storage_error("could not generate an encryption nonce"))?;
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: &self.associated_data,
                },
            )
            .map_err(|_| storage_error("could not encrypt OAuth credentials"))?;
        let mut output = Vec::with_capacity(1 + NONCE_LEN + ciphertext.len());
        output.push(FORMAT_VERSION);
        output.extend_from_slice(&nonce);
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    fn open(&self, encrypted: &[u8]) -> Result<Vec<u8>, AuthError> {
        if encrypted.len() <= 1 + NONCE_LEN || encrypted[0] != FORMAT_VERSION {
            return Err(storage_error("OAuth credential file has an invalid format"));
        }
        let cipher = Aes256Gcm::new_from_slice(self.key.as_ref())
            .map_err(|_| storage_error("invalid encryption key"))?;
        cipher
            .decrypt(
                Nonce::from_slice(&encrypted[1..1 + NONCE_LEN]),
                Payload {
                    msg: &encrypted[1 + NONCE_LEN..],
                    aad: &self.associated_data,
                },
            )
            .map_err(|_| storage_error("could not decrypt OAuth credentials"))
    }

    async fn write_atomically(&self, contents: &[u8]) -> Result<(), AuthError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| storage_error("OAuth credential path has no parent"))?;
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| io_error("create OAuth credential directory", error))?;
        set_owner_only_directory(parent).await?;

        let mut suffix = [0_u8; 8];
        getrandom::fill(&mut suffix)
            .map_err(|_| storage_error("could not generate a temporary filename"))?;
        let temporary = self.path.with_extension(format!("tmp-{}", hex(&suffix)));
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(&temporary)
            .await
            .map_err(|error| io_error("create OAuth credential file", error))?;
        file.write_all(contents)
            .await
            .map_err(|error| io_error("write OAuth credentials", error))?;
        file.sync_all()
            .await
            .map_err(|error| io_error("flush OAuth credentials", error))?;
        drop(file);
        if let Err(error) = tokio::fs::rename(&temporary, self.path.as_ref()).await {
            let _ = tokio::fs::remove_file(&temporary).await;
            return Err(io_error("replace OAuth credentials", error));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl CredentialStore for EncryptedCredentialStore {
    async fn load(&self) -> Result<Option<StoredCredentials>, AuthError> {
        let _guard = self.lock.lock().await;
        let encrypted = match tokio::fs::read(self.path.as_ref()).await {
            Ok(encrypted) => encrypted,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(io_error("read OAuth credentials", error)),
        };
        let plaintext = self.open(&encrypted)?;
        serde_json::from_slice(&plaintext)
            .map(Some)
            .map_err(|_| storage_error("OAuth credential file contains invalid data"))
    }

    async fn save(&self, credentials: StoredCredentials) -> Result<(), AuthError> {
        let _guard = self.lock.lock().await;
        let plaintext = serde_json::to_vec(&credentials)
            .map_err(|_| storage_error("could not serialize OAuth credentials"))?;
        let encrypted = self.seal(&plaintext)?;
        self.write_atomically(&encrypted).await
    }

    async fn clear(&self) -> Result<(), AuthError> {
        let _guard = self.lock.lock().await;
        match tokio::fs::remove_file(self.path.as_ref()).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(io_error("remove OAuth credentials", error)),
        }
    }
}

fn storage_error(message: &str) -> AuthError {
    AuthError::InternalError(message.to_owned())
}

fn io_error(operation: &str, error: std::io::Error) -> AuthError {
    AuthError::InternalError(format!("failed to {operation}: {error}"))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        },
    )
}

#[cfg(unix)]
async fn set_owner_only_directory(path: &Path) -> Result<(), AuthError> {
    use std::os::unix::fs::PermissionsExt as _;
    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .await
        .map_err(|error| io_error("secure OAuth credential directory", error))
}

#[cfg(not(unix))]
async fn set_owner_only_directory(_path: &Path) -> Result<(), AuthError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_payload_is_bound_to_the_slack_identity() {
        let first = EncryptedCredentialStore::new(
            PathBuf::from("unused"),
            [7; 32],
            "team-a\0user-a".to_owned(),
        );
        let second = EncryptedCredentialStore::new(
            PathBuf::from("unused"),
            [7; 32],
            "team-a\0user-b".to_owned(),
        );
        let plaintext = br#"{"access_token":"secret"}"#;
        let encrypted = first.seal(plaintext).unwrap();

        assert_ne!(encrypted, plaintext);
        assert_eq!(first.open(&encrypted).unwrap(), plaintext);
        assert!(second.open(&encrypted).is_err());
    }

    #[test]
    fn encrypted_payload_rejects_tampering() {
        let store = EncryptedCredentialStore::new(
            PathBuf::from("unused"),
            [9; 32],
            "team\0user".to_owned(),
        );
        let mut encrypted = store.seal(b"credentials").unwrap();
        *encrypted.last_mut().unwrap() ^= 1;

        assert!(store.open(&encrypted).is_err());
    }

    #[tokio::test]
    async fn credential_store_round_trips_through_an_encrypted_private_file() {
        let mut suffix = [0_u8; 8];
        getrandom::fill(&mut suffix).unwrap();
        let root = std::env::temp_dir().join(format!("shortrib-oauth-test-{}", hex(&suffix)));
        let store = EncryptedCredentialStore::new(root.clone(), [11; 32], "team\0user".to_owned());
        let credentials =
            StoredCredentials::new("registered-client".to_owned(), None, Vec::new(), None);

        store.save(credentials).await.unwrap();
        let encrypted = tokio::fs::read(store.path.as_ref()).await.unwrap();
        assert!(
            !encrypted
                .windows("registered-client".len())
                .any(|window| window == b"registered-client")
        );
        let restored = store.load().await.unwrap().unwrap();
        assert_eq!(restored.client_id, "registered-client");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = tokio::fs::metadata(store.path.as_ref())
                .await
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }

        store.clear().await.unwrap();
        tokio::fs::remove_dir(root).await.unwrap();
    }
}
