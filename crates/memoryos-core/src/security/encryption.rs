use crate::AppError;
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
    key: Vec<u8>,
}

impl DataEncryptor {
    pub fn new(config: EncryptionConfig) -> Result<Self, AppError> {
        let key = hex_decode(&config.key_hex)
            .map_err(|e| AppError::Config(format!("Invalid encryption key: {}", e)))?;

        if key.len() != 32 {
            return Err(AppError::Config(
                "Encryption key must be 32 bytes (64 hex chars)".to_string(),
            ));
        }

        Ok(Self { config, key })
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedPayload, AppError> {
        if !self.config.enabled {
            return Ok(EncryptedPayload {
                nonce: String::new(),
                ciphertext: hex_encode(plaintext),
            });
        }

        let nonce = generate_nonce();

        let ciphertext = xor_encrypt(plaintext, &self.key, &nonce);

        Ok(EncryptedPayload {
            nonce: hex_encode(&nonce),
            ciphertext: hex_encode(&ciphertext),
        })
    }

    pub fn decrypt(&self, payload: &EncryptedPayload) -> Result<Vec<u8>, AppError> {
        if !self.config.enabled || payload.nonce.is_empty() {
            return hex_decode(&payload.ciphertext)
                .map_err(|e| AppError::Internal(format!("Decryption failed: {}", e)));
        }

        let nonce = hex_decode(&payload.nonce)
            .map_err(|e| AppError::Internal(format!("Invalid nonce: {}", e)))?;
        let ciphertext = hex_decode(&payload.ciphertext)
            .map_err(|e| AppError::Internal(format!("Invalid ciphertext: {}", e)))?;

        let plaintext = xor_encrypt(&ciphertext, &self.key, &nonce);

        Ok(plaintext)
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

fn generate_nonce() -> Vec<u8> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let bytes = ts.to_le_bytes();
    bytes[..12].to_vec()
}

fn xor_encrypt(data: &[u8], key: &[u8], nonce: &[u8]) -> Vec<u8> {
    let mut extended_key = Vec::with_capacity(data.len());
    let combined: Vec<u8> = key
        .iter()
        .zip(nonce.iter().cycle())
        .map(|(k, n)| k ^ n)
        .collect();
    while extended_key.len() < data.len() {
        extended_key.extend_from_slice(&combined);
    }
    data.iter()
        .zip(extended_key.iter())
        .map(|(d, k)| d ^ k)
        .collect()
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
}
