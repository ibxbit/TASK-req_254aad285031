use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;

use super::MIN_PASSWORD_LEN;

#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("password must be at least {0} characters")]
    TooShort(usize),
    #[error("hashing error: {0}")]
    Hashing(String),
}

pub fn validate(plain: &str) -> Result<(), PasswordError> {
    if plain.chars().count() < MIN_PASSWORD_LEN {
        return Err(PasswordError::TooShort(MIN_PASSWORD_LEN));
    }
    Ok(())
}

pub fn hash(plain: &str) -> Result<String, PasswordError> {
    validate(plain)?;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| PasswordError::Hashing(e.to_string()))
}

pub fn verify(plain: &str, hash_str: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash_str) else {
        return false;
    };
    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_short() {
        assert!(matches!(validate("short"), Err(PasswordError::TooShort(_))));
    }

    #[test]
    fn hash_and_verify_roundtrip() {
        let h = hash("correct horse battery staple").unwrap();
        assert!(verify("correct horse battery staple", &h));
        assert!(!verify("wrong password guess!!", &h));
    }

    #[test]
    fn verify_rejects_malformed_hash() {
        assert!(!verify("whatever", "not-a-hash"));
    }
}
