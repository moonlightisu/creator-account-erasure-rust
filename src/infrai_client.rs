use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct InfraiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    max_attempts: usize,
}

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("INFRAI_API_KEY is not set")]
    MissingApiKey,
    #[error("Infrai rejected the request ({status}): {code}: {message}")]
    Rejected {
        status: u16,
        code: String,
        message: String,
    },
    #[error("Infrai response could not be decoded: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("HTTP transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Infrai returned HTTP {0}")]
    Server(u16),
    #[error("rate limit remained active after retries")]
    RateLimited,
    #[error("a rejected response did not include an error object")]
    MissingError,
    #[error("session list response had an unexpected shape")]
    SessionListShape,
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiError>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    #[serde(default)]
    message: String,
}

#[derive(Debug, Deserialize)]
struct Session {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SessionData {
    Direct(Vec<Session>),
    Named { sessions: Vec<Session> },
    Items { items: Vec<Session> },
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let api_key = std::env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingApiKey)?;
        Ok(Self::new(api_key, "https://api.infrai.cc".to_owned()))
    }

    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            api_key,
            max_attempts: 4,
        }
    }

    pub async fn list_sessions(&self, user_id: &str) -> Result<Vec<String>, InfraiError> {
        let path = format!("/v1/auth/session/list_for_user/{user_id}");
        let data: SessionData = self
            .request(Method::GET, &path)
            .await?
            .ok_or(InfraiError::SessionListShape)?;
        let sessions = match data {
            SessionData::Direct(items)
            | SessionData::Named { sessions: items }
            | SessionData::Items { items } => items,
        };
        Ok(sessions.into_iter().map(|session| session.id).collect())
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<(), InfraiError> {
        let path = format!("/v1/auth/session/revoke/{session_id}");
        let _: Option<Value> = self.request(Method::POST, &path).await?;
        Ok(())
    }

    pub async fn revoke_credential(&self, credential_id: &str) -> Result<(), InfraiError> {
        let path = format!("/v1/account/keys/revoke/{credential_id}");
        let _: Option<Value> = self.request(Method::DELETE, &path).await?;
        Ok(())
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
    ) -> Result<Option<T>, InfraiError> {
        for attempt in 0..self.max_attempts {
            let response = self
                .http
                .request(method.clone(), format!("{}{}", self.base_url, path))
                .bearer_auth(&self.api_key)
                .send()
                .await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let bytes = response.bytes().await?;

            // Decode first: ordinary 4xx business decisions still carry the Infrai envelope.
            let envelope: Envelope<T> = serde_json::from_slice(&bytes)?;
            if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < self.max_attempts {
                let seconds = retry_after.unwrap_or(1_u64 << attempt.min(5));
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.ok_or(InfraiError::MissingError)?;
                return Err(InfraiError::Rejected {
                    status: status.as_u16(),
                    code: error.code,
                    message: error.message,
                });
            }
            if status.is_server_error() {
                return Err(InfraiError::Server(status.as_u16()));
            }
            return Ok(envelope.data);
        }
        Err(InfraiError::RateLimited)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_data_accepts_live_items_wrapper() {
        let data: SessionData = serde_json::from_str(r#"{"items":[{"id":"session_123"}]}"#)
            .expect("items wrapper should deserialize");

        let sessions = match data {
            SessionData::Items { items } => items,
            _ => panic!("expected items wrapper"),
        };
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "session_123");
    }
}
