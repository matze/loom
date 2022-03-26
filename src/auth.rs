use crate::error::Error;
use jsonwebtoken as jwt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::convert::{From, TryFrom};
use tower_cookies::Cookies;

const ISS: &'static str = "foo.com";

static KEYS: Lazy<Keys> = Lazy::new(|| {
    let secret = std::env::var("LOOM_JWT_SECRET").expect("LOOM_JWT_SECRET must be set");
    secret.as_bytes().into()
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

impl TryFrom<Cookies> for Token {
    type Error = Error;

    fn try_from(cookies: Cookies) -> Result<Self, Self::Error> {
        let cookie = cookies.get("token").ok_or_else(|| Error::InvalidToken)?;
        Token::try_from(cookie.value())
    }
}
