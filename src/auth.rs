use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub enabled: bool,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    pub jwt_secret: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            issuer_url: "http://localhost:8080".into(),
            client_id: String::new(),
            client_secret: String::new(),
            redirect_url: "http://localhost:3000/auth/callback".into(),
            jwt_secret: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Missing authorization header")]
    MissingHeader,
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Token expired")]
    ExpiredToken,
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Extract user_id from a JWT token string.
/// In RED phase, returns Ok("dev-user") for any non-empty token.
pub fn extract_user_id(token: &str, _secret: &str) -> Result<String, AuthError> {
    if token.is_empty() {
        return Err(AuthError::MissingHeader);
    }
    // Parse JWT without verification for RED phase
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::InvalidToken("malformed token".into()));
    }
    // Decode payload (middle part)
    use base64::Engine;
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| AuthError::InvalidToken("base64 decode failed".into()))?;
    let claims: Claims = serde_json::from_slice(&payload_bytes)
        .map_err(|_| AuthError::InvalidToken("invalid claims".into()))?;

    // Check expiration
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    if claims.exp < now {
        return Err(AuthError::ExpiredToken);
    }

    Ok(claims.sub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.issuer_url, "http://localhost:8080");
    }

    #[test]
    fn test_claims_creation() {
        let claims = Claims {
            sub: "user-123".into(),
            email: Some("test@example.com".into()),
            name: Some("Test User".into()),
            exp: 9999999999,
            iat: 1000000000,
        };
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.email.unwrap(), "test@example.com");
    }

    #[test]
    fn test_auth_error_display() {
        let err = AuthError::MissingHeader;
        assert!(err.to_string().contains("Missing"));
        let err = AuthError::InvalidToken("bad sig".into());
        assert!(err.to_string().contains("bad sig"));
        let err = AuthError::ExpiredToken;
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: "user-1".into(),
            email: None,
            name: None,
            exp: 9999999999,
            iat: 1000000000,
        };
        let json = serde_json::to_value(&claims).unwrap();
        assert_eq!(json["sub"], "user-1");
    }

    #[test]
    fn test_extract_user_id_empty_token() {
        let result = extract_user_id("", "secret");
        assert!(matches!(result, Err(AuthError::MissingHeader)));
    }

    #[test]
    fn test_extract_user_id_malformed_token() {
        let result = extract_user_id("not-a-jwt", "secret");
        assert!(matches!(result, Err(AuthError::InvalidToken(_))));
    }

    #[test]
    fn test_extract_user_id_valid_token() {
        // Create a valid JWT: header.payload.signature
        let claims = json!({
            "sub": "user-abc",
            "email": "test@test.com",
            "exp": 9999999999u64,
            "iat": 1000000000u64,
        });
        use base64::Engine;
        let header =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{\"alg\":\"HS256\"}");
        let payload =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string().as_bytes());
        let token = format!("{}.{}.fakesignature", header, payload);
        let result = extract_user_id(&token, "secret");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "user-abc");
    }

    #[test]
    fn test_extract_user_id_expired_token() {
        let claims = json!({
            "sub": "user-abc",
            "exp": 1, // expired long ago
            "iat": 0,
        });
        use base64::Engine;
        let header =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{\"alg\":\"HS256\"}");
        let payload =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string().as_bytes());
        let token = format!("{}.{}.fakesignature", header, payload);
        let result = extract_user_id(&token, "secret");
        assert!(matches!(result, Err(AuthError::ExpiredToken)));
    }
}
