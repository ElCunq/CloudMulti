use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::env;

#[derive(Clone)]
pub struct Crypto {
    cipher: Aes256Gcm,
}

impl std::fmt::Debug for Crypto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Crypto").finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Missing ENCRYPTION_KEY environment variable")]
    MissingKey,
    #[error("Invalid ENCRYPTION_KEY format: must be exactly 32 bytes or 64 hex characters")]
    InvalidKeyLength,
    #[error("Invalid hex formatting in ENCRYPTION_KEY")]
    InvalidHex,
    #[error("Encryption failed")]
    EncryptionError,
    #[error("Decryption failed: corrupted token or wrong key")]
    DecryptionError,
    #[error("Base64 decoding error")]
    Base64Error,
    #[error("Invalid payload length during decryption")]
    InvalidPayloadLength,
}

impl Crypto {
    pub fn from_env() -> Result<Self, CryptoError> {
        let key_str = env::var("ENCRYPTION_KEY").map_err(|_| CryptoError::MissingKey)?;
        Self::from_key_str(&key_str)
    }

    pub fn from_env_or_auto() -> Result<Self, CryptoError> {
        // 1. Try environment variable first (e.g. from Coolify Environment Variables panel or .env)
        if let Ok(key_str) = env::var("ENCRYPTION_KEY") {
            let trimmed = key_str.trim();
            if !trimmed.is_empty() {
                tracing::info!("Loaded master ENCRYPTION_KEY from environment variable.");
                return Self::from_key_str(trimmed);
            }
        }

        // 2. Try persistent file storage (e.g. inside mounted volume /app/data/encryption_key)
        let key_file_path = std::path::Path::new("data/encryption_key");
        if key_file_path.exists() {
            if let Ok(file_content) = std::fs::read_to_string(key_file_path) {
                let trimmed = file_content.trim();
                if !trimmed.is_empty() {
                    tracing::info!("Loaded existing master ENCRYPTION_KEY from persistent volume: data/encryption_key");
                    return Self::from_key_str(trimmed);
                }
            }
        }

        // 3. First boot / auto-generation: Generate 32 secure random bytes (64 hex characters)
        use rand::RngCore;
        let mut random_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut random_bytes);
        let generated_hex = hex::encode(random_bytes);

        // Ensure data directory exists
        let _ = std::fs::create_dir_all("data");

        // Save to persistent file storage so subsequent restarts use the exact same key
        if let Err(e) = std::fs::write(key_file_path, &generated_hex) {
            tracing::warn!("Could not save auto-generated key to data/encryption_key: {}", e);
        }

        // Print highly visible banner for Coolify logs & UI setup
        let banner = format!(
            "\n================================================================================\n\
             [COOLIFY / DOCKER FIRST DEPLOYMENT NOTICE]\n\
             No ENCRYPTION_KEY found in environment variables or persistent storage.\n\
             A secure 256-bit master encryption key has been AUTO-GENERATED on first boot:\n\n\
             ENCRYPTION_KEY={}\n\n\
             -> This key is saved to your persistent volume at: data/encryption_key\n\
             -> TIP FOR COOLIFY: Copy the ENCRYPTION_KEY value above into your Coolify\n\
                service's 'Environment Variables' panel so it is explicitly shown/managed.\n\
             ================================================================================",
            generated_hex
        );
        tracing::warn!("{}", banner);
        println!("{}", banner);

        Self::from_key_str(&generated_hex)
    }

    pub fn from_key_str(key_str: &str) -> Result<Self, CryptoError> {
        let key_bytes = if key_str.len() == 64 && key_str.chars().all(|c| c.is_ascii_hexdigit()) {
            decode_hex(key_str)?
        } else if key_str.as_bytes().len() == 32 {
            key_str.as_bytes().to_vec()
        } else {
            return Err(CryptoError::InvalidKeyLength);
        };

        if key_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyLength);
        }

        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, CryptoError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits / 12 bytes
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| CryptoError::EncryptionError)?;

        let mut payload = Vec::with_capacity(nonce.len() + ciphertext.len());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&ciphertext);

        Ok(STANDARD.encode(&payload))
    }

    pub fn decrypt(&self, encrypted_base64: &str) -> Result<String, CryptoError> {
        let payload = STANDARD
            .decode(encrypted_base64)
            .map_err(|_| CryptoError::Base64Error)?;

        if payload.len() < 12 {
            return Err(CryptoError::InvalidPayloadLength);
        }

        let (nonce_bytes, ciphertext_bytes) = payload.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext_bytes = self
            .cipher
            .decrypt(nonce, ciphertext_bytes)
            .map_err(|_| CryptoError::DecryptionError)?;

        String::from_utf8(plaintext_bytes).map_err(|_| CryptoError::DecryptionError)
    }
}

fn decode_hex(s: &str) -> Result<Vec<u8>, CryptoError> {
    if s.len() % 2 != 0 {
        return Err(CryptoError::InvalidHex);
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| CryptoError::InvalidHex)?;
        bytes.push(byte);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption_roundtrip() {
        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let crypto = Crypto::from_key_str(key).expect("Should create crypto instance");

        let plaintext = "my_super_secret_cloudflare_api_token_12345";
        let encrypted = crypto.encrypt(plaintext).expect("Should encrypt");
        assert_ne!(plaintext, encrypted);

        let decrypted = crypto.decrypt(&encrypted).expect("Should decrypt");
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_different_nonces() {
        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let crypto = Crypto::from_key_str(key).unwrap();

        let plaintext = "test_token";
        let enc1 = crypto.encrypt(plaintext).unwrap();
        let enc2 = crypto.encrypt(plaintext).unwrap();
        assert_ne!(enc1, enc2);

        assert_eq!(crypto.decrypt(&enc1).unwrap(), plaintext);
        assert_eq!(crypto.decrypt(&enc2).unwrap(), plaintext);
    }

    #[test]
    fn test_invalid_key_length() {
        assert!(Crypto::from_key_str("short").is_err());
    }

    #[test]
    fn test_wrong_key_decryption() {
        let crypto1 = Crypto::from_key_str("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap();
        let crypto2 = Crypto::from_key_str("fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210").unwrap();

        let enc = crypto1.encrypt("secret").unwrap();
        assert!(crypto2.decrypt(&enc).is_err());
    }

    #[test]
    fn test_auto_key_generation_and_persistence() {
        // Temporarily clear env variable for test if set
        let orig_env = env::var("ENCRYPTION_KEY");
        env::remove_var("ENCRYPTION_KEY");

        // Use a test-specific file path or clean up existing one if possible
        // To be safe in parallel tests, from_env_or_auto uses "data/encryption_key".
        let _ = Crypto::from_env_or_auto().expect("Should auto-generate or load");
        assert!(std::path::Path::new("data/encryption_key").exists());

        // Restore orig env
        if let Ok(val) = orig_env {
            env::set_var("ENCRYPTION_KEY", val);
        }
    }
}
