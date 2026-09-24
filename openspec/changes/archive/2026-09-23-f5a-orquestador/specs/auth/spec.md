## ADDED Requirements

### Requirement: PocketID OIDC authentication
Alfred must authenticate users via PocketID as a self-hosted OIDC provider.

**Contracts:**
```rust
pub struct AuthConfig {
    pub enabled: bool,           // Allow disabling auth in dev
    pub issuer_url: String,      // PocketID issuer (e.g., http://pocketid:8080)
    pub client_id: String,       // Alfred's client ID in PocketID
    pub client_secret: String,   // Alfred's client secret
    pub redirect_url: String,    // Post-login redirect
    pub jwt_secret: String,      // For verifying JWTs
}

pub struct Claims {
    pub sub: String,             // PocketID user ID
    pub email: Option<String>,
    pub name: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

pub struct AuthMiddleware;

impl AuthMiddleware {
    /// Extract user_id from JWT in Authorization header
    pub fn extract_user_id(req: &Request) -> Result<String, AuthError>;
}

pub enum AuthError {
    MissingHeader,
    InvalidToken(String),
    ExpiredToken,
    ConfigError(String),
}

// GET /api/auth/login — redirects to PocketID
// GET /api/auth/callback — handles OIDC callback, sets JWT cookie
// GET /api/auth/logout — clears session
// GET /api/auth/me — returns current user claims
```

**Scenarios:**
#### Scenario: Valid JWT returns user_id
Given a valid JWT in the Authorization header
When `extract_user_id()` is called
Then the user's PocketID sub is returned

#### Scenario: Missing header returns error
Given a request without Authorization header
When `extract_user_id()` is called
Then AuthError::MissingHeader is returned

#### Scenario: Expired JWT returns error
Given an expired JWT (past exp)
When `extract_user_id()` is called
Then AuthError::ExpiredToken is returned

#### Scenario: Auth disabled in dev mode
Given AuthConfig with enabled: false
When any request arrives without a token
Then the request proceeds with a default "dev-user" ID