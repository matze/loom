use crate::error::Error;
use axum::extract::{FromRequest, RequestParts, TypedHeader};
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use jsonwebtoken as jwt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::convert::From;

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
}

pub fn validate(token: &str) -> Result<(), Error> {
    let token = jwt::decode::<Claims>(token, &KEYS.decoding, &jwt::Validation::default())
        .map_err(|_| Error::InvalidToken)?;

    if token.claims.iss != ISS {
        Err(Error::WrongCredentials)
    } else {
        Ok(())
    }
}

impl From<Token> for String {
    fn from(token: Token) -> Self {
        token.0
    }
}

#[axum::async_trait]
impl<B> FromRequest<B> for Token
where
    B: Send,
{
    type Rejection = Error;

    async fn from_request(parts: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request(parts)
                .await
                .map_err(|_| Error::InvalidToken)?;

        let token = bearer.token();
        validate(token)?;
        Ok(Token(token.to_string()))
    }
}
