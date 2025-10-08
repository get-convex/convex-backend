use std::{
    collections::HashMap,
    future::Future,
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use common::types::MemberId;
use errors::ErrorMetadata;
use oauth2::{
    HttpRequest,
    HttpResponse,
};
use serde::{
    Deserialize,
    Serialize,
};
use tokio::time::timeout;

/// Maps WorkOS identity providers to Auth0-compatible subject formats
pub fn map_workos_identities_to_subjects(
    workos_user_id: &str,
    identities: &[WorkOSIdentity],
) -> anyhow::Result<Vec<String>> {
    match identities.len() {
        // If there are no identities
        0 => Ok(vec![workos_user_id.to_string()]),
        _ => identities
            .iter()
            .map(|identity| -> anyhow::Result<String> {
                let mapped_provider = match identity.provider.as_str() {
                    "GithubOAuth" => "github",
                    "GoogleOAuth" => "google-oauth2",
                    "VercelOAuth" => "vercel",
                    _ => anyhow::bail!("Unsupported provider: {}", identity.provider),
                };

                // This is the old format of Auth0 subjects for backwards compatability
                let subject = format!("{}|{}", mapped_provider, identity.idp_id);
                Ok(subject)
            })
            .collect::<Result<Vec<String>, _>>(),
    }
}

#[derive(Debug, Deserialize)]
pub struct WorkOSIdentity {
    pub idp_id: String,
    pub provider: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkOSUser {
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

const APPLICATION_JSON: http::HeaderValue = http::HeaderValue::from_static("application/json");

// Timeout for external WorkOS API calls
const WORKOS_API_TIMEOUT: Duration = Duration::from_secs(5);

fn format_workos_error(operation: &str, status: http::StatusCode, response_body: &[u8]) -> String {
    let body_str = String::from_utf8_lossy(response_body);
    let truncated_body = if body_str.len() > 1000 {
        format!("{}...", &body_str[..1000])
    } else {
        body_str.to_string()
    };

    format!(
        "WorkOS {} API returned HTTP {} {}: {}",
        operation,
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown"),
        truncated_body
    )
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorkOSTeamResponse {
    /// always "team"
    pub object: String,
    /// like "team_01K58C005DSAQCZSX84FFWMT5G"
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorkOSEnvironmentResponse {
    /// always "environment"
    pub object: String,
    /// like "environment_01K5DJZTWGXWJMFSMHY3HCXK8N"
    pub id: String,
    pub name: String,
    pub client_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorkOSAPIKeyResponse {
    /// always "api_key"
    pub object: String,
    /// like "key_01K5DJZVGQ4JM58YS3VC5C5QD3"
    pub id: String,
    pub name: String,
    pub expires_at: Option<String>,
    /// like 'sk_test_a2V5XzAxSzVESlpWR1E0Sk01OFlTM1ZDNUM1UUQzLEIzZkcxNkVxR0swanZVQUZaTXN4VmNWTng'
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkOSErrorResponse {
    pub code: String,
    pub message: String,
}

#[async_trait]
pub trait WorkOSClient: Send + Sync {
    async fn fetch_identities(&self, user_id: &str) -> anyhow::Result<Vec<WorkOSIdentity>>;
    async fn fetch_user(&self, user_id: &str) -> anyhow::Result<WorkOSUser>;
    async fn delete_user(&self, user_id: &str) -> anyhow::Result<()>;
    async fn update_user_metadata(&self, user_id: &str, member_id: MemberId) -> anyhow::Result<()>;
}

// Separate trait for WorkOS Platform API operations (requires different API
// key)
#[async_trait]
pub trait WorkOSPlatformClient: Send + Sync {
    async fn create_team(
        &self,
        admin_email: &str,
        team_name: &str,
    ) -> anyhow::Result<WorkOSTeamResponse>;
    async fn create_environment(
        &self,
        workos_team_id: &str,
        environment_name: &str,
    ) -> anyhow::Result<WorkOSEnvironmentResponse>;
    async fn create_api_key(
        &self,
        workos_team_id: &str,
        environment_id: &str,
        key_name: &str,
    ) -> anyhow::Result<WorkOSAPIKeyResponse>;
}

pub struct RealWorkOSClient<F, E>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    api_key: String,
    http_client: Box<dyn Fn(HttpRequest) -> F + Send + Sync + 'static>,
}

impl<F, E> RealWorkOSClient<F, E>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    pub fn new(
        api_key: String,
        http_client: impl Fn(HttpRequest) -> F + Send + Sync + 'static,
    ) -> Self {
        Self {
            api_key,
            http_client: Box::new(http_client),
        }
    }
}

#[async_trait]
impl<F, E> WorkOSClient for RealWorkOSClient<F, E>
where
    F: Future<Output = Result<HttpResponse, E>> + Send + 'static,
    E: std::error::Error + 'static + Send + Sync,
{
    async fn fetch_identities(&self, user_id: &str) -> anyhow::Result<Vec<WorkOSIdentity>> {
        fetch_workos_identities(&self.api_key, user_id, &*self.http_client).await
    }

    async fn fetch_user(&self, user_id: &str) -> anyhow::Result<WorkOSUser> {
        fetch_workos_user(&self.api_key, user_id, &*self.http_client).await
    }

    async fn delete_user(&self, user_id: &str) -> anyhow::Result<()> {
        delete_workos_user(&self.api_key, user_id, &*self.http_client).await
    }

    async fn update_user_metadata(&self, user_id: &str, member_id: MemberId) -> anyhow::Result<()> {
        update_workos_user_metadata(&self.api_key, user_id, member_id, &*self.http_client).await
    }
}

pub struct MockWorkOSClient;

impl Default for MockWorkOSClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockWorkOSClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WorkOSClient for MockWorkOSClient {
    async fn fetch_identities(&self, user_id: &str) -> anyhow::Result<Vec<WorkOSIdentity>> {
        if let Some(pipe_pos) = user_id.find('|') {
            let provider = &user_id[..pipe_pos];
            let idp_id = &user_id[pipe_pos + 1..];

            Ok(vec![WorkOSIdentity {
                provider: match provider {
                    "google-oauth2" => "GoogleOAuth",
                    "github" => "GithubOAuth",
                    _ => "Unknown",
                }
                .to_string(),
                idp_id: idp_id.to_string(),
            }])
        } else {
            Ok(vec![])
        }
    }

    async fn fetch_user(&self, _user_id: &str) -> anyhow::Result<WorkOSUser> {
        Ok(WorkOSUser {
            email: "test@example.com".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
        })
    }

    async fn delete_user(&self, _user_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn update_user_metadata(
        &self,
        _user_id: &str,
        _member_id: MemberId,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

// Separate implementation for WorkOS Platform API
pub struct RealWorkOSPlatformClient<F, E>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    platform_api_key: String,
    http_client: Box<dyn Fn(HttpRequest) -> F + Send + Sync + 'static>,
}

impl<F, E> RealWorkOSPlatformClient<F, E>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    pub fn new(
        platform_api_key: String,
        http_client: impl Fn(HttpRequest) -> F + Send + Sync + 'static,
    ) -> Self {
        Self {
            platform_api_key,
            http_client: Box::new(http_client),
        }
    }
}

#[async_trait]
impl<F, E> WorkOSPlatformClient for RealWorkOSPlatformClient<F, E>
where
    F: Future<Output = Result<HttpResponse, E>> + Send + 'static,
    E: std::error::Error + 'static + Send + Sync,
{
    async fn create_team(
        &self,
        admin_email: &str,
        team_name: &str,
    ) -> anyhow::Result<WorkOSTeamResponse> {
        create_workos_team(
            &self.platform_api_key,
            admin_email,
            team_name,
            &*self.http_client,
        )
        .await
    }

    async fn create_environment(
        &self,
        workos_team_id: &str,
        environment_name: &str,
    ) -> anyhow::Result<WorkOSEnvironmentResponse> {
        create_workos_environment(
            &self.platform_api_key,
            workos_team_id,
            environment_name,
            &*self.http_client,
        )
        .await
    }

    async fn create_api_key(
        &self,
        workos_team_id: &str,
        environment_id: &str,
        key_name: &str,
    ) -> anyhow::Result<WorkOSAPIKeyResponse> {
        create_workos_api_key(
            &self.platform_api_key,
            workos_team_id,
            environment_id,
            key_name,
            &*self.http_client,
        )
        .await
    }
}

pub struct MockWorkOSPlatformClient;

impl Default for MockWorkOSPlatformClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockWorkOSPlatformClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WorkOSPlatformClient for MockWorkOSPlatformClient {
    async fn create_team(
        &self,
        _admin_email: &str,
        team_name: &str,
    ) -> anyhow::Result<WorkOSTeamResponse> {
        Ok(WorkOSTeamResponse {
            object: "team".to_string(),
            id: "team_mock123".to_string(),
            name: team_name.to_string(),
            created_at: "2024-01-01T00:00:00.000Z".to_string(),
            updated_at: "2024-01-01T00:00:00.000Z".to_string(),
        })
    }

    async fn create_environment(
        &self,
        _workos_team_id: &str,
        environment_name: &str,
    ) -> anyhow::Result<WorkOSEnvironmentResponse> {
        Ok(WorkOSEnvironmentResponse {
            object: "environment".to_string(),
            id: "env_mock123".to_string(),
            name: environment_name.to_string(),
            client_id: "client_mock123".to_string(),
        })
    }

    async fn create_api_key(
        &self,
        _workos_team_id: &str,
        _environment_id: &str,
        key_name: &str,
    ) -> anyhow::Result<WorkOSAPIKeyResponse> {
        Ok(WorkOSAPIKeyResponse {
            object: "api_key".to_string(),
            id: "key_mock123".to_string(),
            name: key_name.to_string(),
            expires_at: None,
            value: "sk_test_mock_key_value".to_string(),
            created_at: "2024-01-01T00:00:00.000Z".to_string(),
            updated_at: "2024-01-01T00:00:00.000Z".to_string(),
        })
    }
}

pub async fn fetch_workos_identities<F, E>(
    api_key: &str,
    user_id: &str,
    http_client: &(impl Fn(HttpRequest) -> F + 'static + ?Sized),
) -> anyhow::Result<Vec<WorkOSIdentity>>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    let url = format!("https://api.workos.com/user_management/users/{user_id}/identities");

    let request = http::Request::builder()
        .uri(&url)
        .method(http::Method::GET)
        .header(http::header::AUTHORIZATION, format!("Bearer {api_key}"))
        .header(http::header::ACCEPT, APPLICATION_JSON)
        .header(http::header::CONTENT_TYPE, APPLICATION_JSON)
        .body(vec![])?;

    let response = http_client(request)
        .await
        .map_err(|e| anyhow::anyhow!("Could not fetch WorkOS identities: {}", e))?;

    if response.status() != http::StatusCode::OK {
        let status = response.status();
        let response_body = response.into_body();
        anyhow::bail!(format_workos_error(
            "list identities",
            status,
            &response_body
        ));
    }

    let response_body = response.into_body();
    let identities: Vec<WorkOSIdentity> =
        serde_json::from_slice(&response_body).with_context(|| {
            format!(
                "Invalid WorkOS identities response: {}",
                String::from_utf8_lossy(&response_body)
            )
        })?;

    Ok(identities)
}

pub async fn fetch_workos_user<F, E>(
    api_key: &str,
    user_id: &str,
    http_client: &(impl Fn(HttpRequest) -> F + 'static + ?Sized),
) -> anyhow::Result<WorkOSUser>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    let url = format!("https://api.workos.com/user_management/users/{user_id}");

    let request = http::Request::builder()
        .uri(&url)
        .method(http::Method::GET)
        .header(http::header::AUTHORIZATION, format!("Bearer {api_key}"))
        .header(http::header::ACCEPT, APPLICATION_JSON)
        .header(http::header::CONTENT_TYPE, APPLICATION_JSON)
        .body(vec![])?;

    let response = http_client(request)
        .await
        .map_err(|e| anyhow::anyhow!("Could not fetch WorkOS user: {}", e))?;

    if response.status() != http::StatusCode::OK {
        let status = response.status();
        let response_body = response.into_body();
        anyhow::bail!(format_workos_error("get user", status, &response_body));
    }

    let response_body = response.into_body();
    let user: WorkOSUser = serde_json::from_slice(&response_body).with_context(|| {
        format!(
            "Invalid WorkOS user response: {}",
            String::from_utf8_lossy(&response_body)
        )
    })?;

    Ok(user)
}

pub async fn delete_workos_user<F, E>(
    api_key: &str,
    user_id: &str,
    http_client: &(impl Fn(HttpRequest) -> F + 'static + ?Sized),
) -> anyhow::Result<()>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    let url = format!("https://api.workos.com/user_management/users/{user_id}");

    let request = http::Request::builder()
        .uri(&url)
        .method(http::Method::DELETE)
        .header(http::header::AUTHORIZATION, format!("Bearer {api_key}"))
        .header(http::header::ACCEPT, APPLICATION_JSON)
        .body(vec![])?;

    let response = http_client(request)
        .await
        .map_err(|e| anyhow::anyhow!("Could not delete WorkOS user: {}", e))?;

    if response.status() != http::StatusCode::OK {
        if response.status() == http::StatusCode::NOT_FOUND {
            return Ok(());
        }
        let status = response.status();
        let response_body = response.into_body();
        anyhow::bail!(format_workos_error("delete user", status, &response_body));
    }

    Ok(())
}

#[derive(Serialize)]
struct WorkOSUserMetadata {
    convex_member_id: String,
}

pub async fn update_workos_user_metadata<F, E>(
    api_key: &str,
    user_id: &str,
    member_id: MemberId,
    http_client: &(impl Fn(HttpRequest) -> F + 'static + ?Sized),
) -> anyhow::Result<()>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    let url = format!("https://api.workos.com/user_management/users/{user_id}");

    let metadata = WorkOSUserMetadata {
        convex_member_id: member_id.to_string(),
    };

    let mut update_data = HashMap::new();
    update_data.insert("metadata", metadata);

    let request_body = serde_json::to_vec(&update_data)
        .map_err(|e| anyhow::anyhow!("Failed to serialize update data: {}", e))?;

    let request = http::Request::builder()
        .uri(&url)
        .method(http::Method::PUT)
        .header(http::header::AUTHORIZATION, format!("Bearer {api_key}"))
        .header(http::header::ACCEPT, APPLICATION_JSON)
        .header(http::header::CONTENT_TYPE, APPLICATION_JSON)
        .body(request_body)?;

    let response = http_client(request)
        .await
        .map_err(|e| anyhow::anyhow!("Could not update WorkOS user metadata: {}", e))?;

    if response.status() != http::StatusCode::OK {
        let status = response.status();
        let response_body = response.into_body();
        anyhow::bail!(format_workos_error("update user", status, &response_body));
    }

    Ok(())
}

pub async fn create_workos_team<F, E>(
    api_key: &str,
    admin_email: &str,
    team_name: &str,
    http_client: &(impl Fn(HttpRequest) -> F + 'static + ?Sized),
) -> anyhow::Result<WorkOSTeamResponse>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    #[derive(Serialize)]
    struct CreateTeamRequest {
        admin_email: String,
        name: String,
    }

    let request_body = CreateTeamRequest {
        admin_email: admin_email.to_string(),
        name: team_name.to_string(),
    };

    let request = http::Request::builder()
        .uri("https://api.workos.com/platform/teams")
        .method(http::Method::POST)
        .header(http::header::AUTHORIZATION, format!("Bearer {api_key}"))
        .header(http::header::CONTENT_TYPE, APPLICATION_JSON)
        .header(http::header::ACCEPT, APPLICATION_JSON)
        .body(serde_json::to_vec(&request_body)?)?;

    let response = timeout(WORKOS_API_TIMEOUT, http_client(request))
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "WorkOS API call timed out after {}s",
                WORKOS_API_TIMEOUT.as_secs()
            )
        })?
        .map_err(|e| anyhow::anyhow!("Could not create WorkOS team: {}", e))?;

    if response.status() == http::StatusCode::CONFLICT {
        let response_body = response.into_body();

        if let Ok(error_response) = serde_json::from_slice::<WorkOSErrorResponse>(&response_body) {
            if error_response.code == "user_already_exists" {
                // This will be special-cased in scripts.
                anyhow::bail!(ErrorMetadata::bad_request(
                    "WorkosAccountAlreadyExistsWithThisEmail",
                    format!("A WorkOS account already exists with the email: {admin_email}")
                ));
            }
        }

        let status = http::StatusCode::CONFLICT;
        anyhow::bail!(format_workos_error(
            "create team (conflict)",
            status,
            &response_body
        ));
    }

    if !response.status().is_success() {
        let status = response.status();
        let response_body = response.into_body();
        anyhow::bail!(format_workos_error("create team", status, &response_body));
    }

    let response_body = response.into_body();
    let team: WorkOSTeamResponse = serde_json::from_slice(&response_body).with_context(|| {
        format!(
            "Invalid WorkOS team response: {}",
            String::from_utf8_lossy(&response_body)
        )
    })?;

    Ok(team)
}

pub async fn create_workos_environment<F, E>(
    api_key: &str,
    workos_team_id: &str,
    environment_name: &str,
    http_client: &(impl Fn(HttpRequest) -> F + 'static + ?Sized),
) -> anyhow::Result<WorkOSEnvironmentResponse>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    #[derive(Serialize)]
    struct CreateEnvironmentRequest {
        name: String,
    }

    let request_body = CreateEnvironmentRequest {
        name: environment_name.to_string(),
    };

    let url = format!("https://api.workos.com/platform/teams/{workos_team_id}/environments",);

    let request = http::Request::builder()
        .uri(&url)
        .method(http::Method::POST)
        .header(http::header::AUTHORIZATION, format!("Bearer {api_key}"))
        .header(http::header::CONTENT_TYPE, APPLICATION_JSON)
        .header(http::header::ACCEPT, APPLICATION_JSON)
        .body(serde_json::to_vec(&request_body)?)?;

    let response = timeout(WORKOS_API_TIMEOUT, http_client(request))
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "WorkOS API call timed out after {}s",
                WORKOS_API_TIMEOUT.as_secs()
            )
        })?
        .map_err(|e| anyhow::anyhow!("Could not create WorkOS environment: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let response_body = response.into_body();
        anyhow::bail!(format_workos_error(
            "create environment",
            status,
            &response_body
        ));
    }

    let response_body = response.into_body();
    let environment: WorkOSEnvironmentResponse = serde_json::from_slice(&response_body)
        .with_context(|| {
            format!(
                "Invalid WorkOS environment response: {}",
                String::from_utf8_lossy(&response_body)
            )
        })?;

    Ok(environment)
}

pub async fn create_workos_api_key<F, E>(
    api_key: &str,
    workos_team_id: &str,
    environment_id: &str,
    key_name: &str,
    http_client: &(impl Fn(HttpRequest) -> F + 'static + ?Sized),
) -> anyhow::Result<WorkOSAPIKeyResponse>
where
    F: Future<Output = Result<HttpResponse, E>>,
    E: std::error::Error + 'static + Send + Sync,
{
    #[derive(Serialize)]
    struct CreateAPIKeyRequest {
        name: String,
        expires_at: Option<String>,
    }

    let request_body = CreateAPIKeyRequest {
        name: key_name.to_string(),
        expires_at: None,
    };

    let url = format!(
        "https://api.workos.com/platform/teams/{workos_team_id}/environments/{environment_id}/api_keys",
    );

    let request = http::Request::builder()
        .uri(&url)
        .method(http::Method::POST)
        .header(http::header::AUTHORIZATION, format!("Bearer {api_key}"))
        .header(http::header::CONTENT_TYPE, APPLICATION_JSON)
        .header(http::header::ACCEPT, APPLICATION_JSON)
        .body(serde_json::to_vec(&request_body)?)?;

    let response = timeout(WORKOS_API_TIMEOUT, http_client(request))
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "WorkOS API call timed out after {}s",
                WORKOS_API_TIMEOUT.as_secs()
            )
        })?
        .map_err(|e| anyhow::anyhow!("Could not create WorkOS API key: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let response_body = response.into_body();
        anyhow::bail!(format_workos_error(
            "create API key",
            status,
            &response_body
        ));
    }

    let response_body = response.into_body();
    let api_key_response: WorkOSAPIKeyResponse = serde_json::from_slice(&response_body)
        .with_context(|| {
            format!(
                "Invalid WorkOS API key response: {}",
                String::from_utf8_lossy(&response_body)
            )
        })?;

    Ok(api_key_response)
}

#[cfg(test)]
mod tests {
    use super::WorkOSIdentity;

    #[tokio::test]
    async fn test_workos_identity_parsing() -> anyhow::Result<()> {
        // Test that we can parse the WorkOS identity response format
        let response_json = r#"[{"idp_id":"9063110","type":"OAuth","provider":"GithubOAuth"},{"idp_id":"112960081753601695488","type":"OAuth","provider":"GoogleOAuth"}]"#;

        let identities: Vec<WorkOSIdentity> = serde_json::from_str(response_json)?;

        assert_eq!(identities.len(), 2);
        assert_eq!(identities[0].idp_id, "9063110");
        assert_eq!(identities[0].provider, "GithubOAuth");
        assert_eq!(identities[1].idp_id, "112960081753601695488");
        assert_eq!(identities[1].provider, "GoogleOAuth");

        Ok(())
    }
}
