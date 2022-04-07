use crate::error::Error;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum_extra::extract::CookieJar;
use jsonwebtoken as jwt;
use once_cell::sync::Lazy;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::convert::{From, TryFrom};

const ISS: &str = "foo.com";

static KEYS: Lazy<Keys> = Lazy::new(|| {
    let mut secret = [0u8; 256];
    OsRng.fill_bytes(&mut secret);
    secret.as_ref().into()
});

struct Keys {
    encoding: jwt::EncodingKey,
    decoding: jwt::DecodingKey,
}

impl From<&[u8]> for Keys {
    fn from(secret: &[u8]) -> Self {
        Self {
            encoding: jwt::EncodingKey::from_secret(secret),
            decoding: jwt::DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Claims {
    exp: usize,
    iss: String,
    sub: String,
}

#[derive(Clone)]
pub(crate) struct Token(String);

impl Token {
    pub(crate) fn new(user: &str) -> Result<Self, Error> {
        let claims = Claims {
            exp: 2000000000,
            iss: ISS.to_string(),
            sub: user.to_string(),
        };

        let token = jwt::encode(&jwt::Header::default(), &claims, &KEYS.encoding)?;

        Ok(Self(token))
    }

    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<&str> for Token {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let token = jwt::decode::<Claims>(value, &KEYS.decoding, &jwt::Validation::default())
            .map_err(|_| Error::InvalidToken)?;

        if token.claims.iss != ISS {
            Err(Error::WrongCredentials)
        } else {
            Ok(Token(value.to_string()))
        }
    }
}

impl TryFrom<CookieJar> for Token {
    type Error = Error;

    fn try_from(cookies: CookieJar) -> Result<Self, Self::Error> {
        let cookie = cookies.get("token").ok_or(Error::InvalidToken)?;
        Token::try_from(cookie.value())
    }
}

pub fn hash_secret(secret: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);

    argon2::Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

pub fn verify_secret(hash: &str, secret: &str) -> bool {
    let hash = PasswordHash::new(hash).unwrap();

    argon2::Argon2::default()
        .verify_password(secret.as_bytes(), &hash)
        .is_ok()
}
