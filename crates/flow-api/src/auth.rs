//! Módulo de autenticación y autorización.

use axum::{
  async_trait,
  extract::FromRequestParts,
  http::{header::AUTHORIZATION, request::Parts, StatusCode},
  response::{IntoResponse, Response},
  Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Display;
use uuid::Uuid;

use crate::config::CONFIG;

pub static KEYS: Lazy<Keys> = Lazy::new(|| {
  let secret = CONFIG.jwt_secret.as_bytes();
  Keys { encoding: EncodingKey::from_secret(secret), decoding: DecodingKey::from_secret(secret) }
});

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
  pub sub: Uuid,
  pub exp: usize,
  pub iat: usize,
}

#[derive(Debug, Serialize)]
pub struct AuthBody {
  token: String,
  token_type: String,
}

impl AuthBody {
  pub fn new(token: String) -> Self {
    Self { token, token_type: "Bearer".to_string() }
  }
}

#[derive(Debug)]
pub enum AuthError {
  WrongCredentials,
  MissingCredentials,
  TokenCreation,
  InvalidToken,
}

impl IntoResponse for AuthError {
  fn into_response(self) -> Response {
    let (status, error_message) = match self {
      AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
      AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
      AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
      AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
    };
    let body = Json(json!({
                        "error": error_message,
                    }));
    (status, body).into_response()
  }
}

pub struct Keys {
  pub encoding: EncodingKey,
  pub decoding: DecodingKey,
}

pub fn create_jwt(uid: Uuid) -> Result<String, AuthError> {
  let now = chrono::Utc::now();
  let iat = now.timestamp() as usize;
  let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;
  let claims = Claims { sub: uid, exp, iat };
  encode(&Header::default(), &claims, &KEYS.encoding).map_err(|_| AuthError::TokenCreation)
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims where S: Send + Sync
{
  type Rejection = AuthError;

  async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
    // Extract the token from the Authorization header
    let auth_header = parts.headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()).ok_or(AuthError::MissingCredentials)?;

    let token = auth_header.strip_prefix("Bearer ").ok_or(AuthError::InvalidToken)?;

    // Decode the user data
    let token_data = decode::<Claims>(token, &KEYS.decoding, &Validation::default()).map_err(|_| AuthError::InvalidToken)?;

    Ok(token_data.claims)
  }
}

impl Display for Claims {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.sub)
  }
}
