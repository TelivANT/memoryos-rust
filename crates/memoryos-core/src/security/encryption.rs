use crate::AppError;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct EncryptionConfig {
    pub enabled: bool,
    pub key_hex: String,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            key_hex: "0".repeat(64),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub nonce: String,
    pub ciphertext: String,
}

pub struct DataEncryptor {
    config: EncryptionConfig,
    cipher: Aes256Gcm,
}

impl DataEncryptor {
    pub fn new(config: EncryptionConfig) -> Result<Self, AppError> {
        let key_bytes = hex_decode(&config.key_hex)
            .map_err(|e| AppError::Config(format!("Invalid encryption key: {}", e)))?;

        if key_bytes.len() != 32 {
            return Err(AppError::Config(
                "Encryption key must be 32 bytes (64 hex chars)".to_string(),
            ));
        }

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        Ok(Self { config, cipher })
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedPayload, AppError> {
        if !self.config.enabled {
            return Ok(EncryptedPayload {
                nonce: String::new(),
                ciphertext: hex_encode(plaintext),
            });
        }

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| AppError::Internal(format!("Encryption failed: {}", e)))?;

        Ok(EncryptedPayload {
            nonce: hex_encode(&nonce_bytes),
            ciphertext: hex_encode(&ciphertext),
        })
    }

    pub fn decrypt(&self, payload: &EncryptedPayload) -> Result<Vec<u8>, AppError> {
        if !self.config.enabled || payload.nonce.is_empty() {
            return hex_decode(&payload.ciphertext)
                .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)));
        }

        let nonce_bytes = hex_decode(&payload.nonce)
            .map_err(|e| AppError::Internal(format!("Invalid nonce: {}", e)))?;
        let ciphertext = hex_decode(&payload.ciphertext)
            .map_err(|e| AppError::Internal(format!("Invalid ciphertext: {}", e)))?;

        let nonce = Nonce::from_slice(&nonce_bytes);
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)))?;

        Ok(plaintext)
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Invalid hex string length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex at position {}: {}", i, e))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_enabled() {
        let config = EncryptionConfig {
            enabled: true,
            key_hex: "a".repeat(64),
        };
        let encryptor = DataEncryptor::new(config).unwrap();
        let plaintext = b"Hello, World!";
        let encrypted = encryptor.encrypt(plaintext).unwrap();
        assert!(!encrypted.nonce.is_empty());
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_disabled() {
        let config = EncryptionConfig {
            enabled: false,
            key_hex: "0".repeat(64),
        };
        let encryptor = DataEncryptor::new(config).unwrap();
        let plaintext = b"Hello, World!";
        let encrypted = encryptor.encrypt(plaintext).unwrap();
        assert!(encrypted.nonce.is_empty());
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_invalid_key_length() {
        let config = EncryptionConfig {
            enabled: true,
            key_hex: "aa".to_string(),
        };
        assert!(DataEncryptor::new(config).is_err());
    }

    #[test]
    fn test_hex_roundtrip() {
        let data = vec![0x00, 0xff, 0xab, 0xcd];
        let hex = hex_encode(&data);
        let decoded = hex_decode(&hex).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_unique_nonces() {
        let config = EncryptionConfig {
            enabled: true,
            key_hex: "b".repeat(64),
        };
        let encryptor = DataEncryptor::new(config).unwrap();
        let plaintext = b"same data";
        let e1 = encryptor.encrypt(plaintext).unwrap();
        let e2 = encryptor.encrypt(plaintext).unwrap();
        assert_ne!(e1.nonce, e2.nonce);
        assert_ne!(e1.ciphertext, e2.ciphertext);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let config = EncryptionConfig {
            enabled: true,
            key_hex: "c".repeat(64),
        };
        let encryptor = DataEncryptor::new(config).unwrap();
        let encrypted = encryptor.encrypt(b"secret data").unwrap();
        let tampered = EncryptedPayload {
            nonce: encrypted.nonce,
            ciphertext: "ff".repeat(encrypted.ciphertext.len() / 2),
        };
        assert!(encryptor.decrypt(&tampered).is_err());
    }

    #[test]
    fn test_large_payload() {
        let config = EncryptionConfig {
            enabled: true,
            key_hex: "d".repeat(64),
        };
        let encryptor = DataEncryptor::new(config).unwrap();
        let plaintext = vec![0x42u8; 65536];
        let encrypted = encryptor.encrypt(&plaintext).unwrap();
        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
