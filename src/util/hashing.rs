use sha2::{Digest, Sha256};

pub fn sha256_text(text: &str) -> String {
    sha256_bytes(text.as_bytes())
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
