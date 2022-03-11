use crate::error::Error;
use axum::extract::{FromRequest, RequestParts, TypedHeader};
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use jsonwebtoken as jwt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::convert::From;

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
    user: String,
}

pub(crate) struct Token(String);

impl Token {
    pub(crate) fn new(user: &str) -> Result<Self, Error> {
        let claims = Claims {
            exp: 2000000000,
            iss: "foobar".to_string(),
            user: user.to_string(),
        };

        let token = jwt::encode(&jwt::Header::default(), &claims, &KEYS.encoding)?;

        Ok(Self(token))
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

        let _ = jwt::decode::<Claims>(bearer.token(), &KEYS.decoding, &jwt::Validation::default())
            .map_err(|_| Error::InvalidToken)?;

        Ok(Token(bearer.token().to_string()))
    }
}