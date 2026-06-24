use crate::github::api::client::GithubRepositoryClient;
use crate::github::{GithubRepoName, GithubUser, prepare_octocrab_client};
use anyhow::Context;
use axum_session::SessionNullSession;
use base64::Engine;
use base64::prelude::BASE64_URL_SAFE;
use octocrab::Octocrab;
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashMap;
use std::fmt::Write;

#[derive(serde::Deserialize)]
#[serde(transparent)]
pub struct AccessCode(String);

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct AccessToken(String);

/// Client that handles OAuth authentication used for rollups.
/// It is able to provide a GitHub client authenticated as a given GitHub user.
#[derive(Clone)]
pub struct OAuthClient {
    config: OAuthConfig,
    github_base_url: String,
    github_api_base_url: String,
}

impl OAuthClient {
    pub fn new(config: OAuthConfig, github_base_url: String, github_api_base_url: String) -> Self {
        Self {
            config,
            github_base_url,
            github_api_base_url,
        }
    }

    pub fn config(&self) -> &OAuthConfig {
        &self.config
    }

    /// Upgrades a GitHub access token from the OAuth callback into a GitHub access token.
    pub async fn get_github_access_token(
        &self,
        AccessCode(code): AccessCode,
    ) -> anyhow::Result<AccessToken> {
        tracing::info!("Exchanging OAuth code for access token");
        let client = reqwest::Client::new();
        let token_response = client
            .post(format!("{}/login/oauth/access_token", self.github_base_url))
            .form(&[
                ("client_id", self.config.client_id()),
                ("client_secret", self.config.client_secret()),
                ("code", &code),
            ])
            .send()
            .await
            .context("Failed to send OAuth token exchange request to GitHub")?
            .text()
            .await
            .context("Failed to read OAuth token response from GitHub")?;

        let oauth_token_params: HashMap<String, String> =
            url::form_urlencoded::parse(token_response.as_bytes())
                .into_owned()
                .collect();
        let access_token = oauth_token_params
            .get("access_token")
            .ok_or_else(|| anyhow::anyhow!("No OAuth access token in response"))?;

        tracing::info!("Retrieved OAuth access token");
        Ok(AccessToken(access_token.to_owned()))
    }

    /// Create an authenticated Octocrab client with the given OAuth access token.
    pub fn get_authenticated_client(
        &self,
        AccessToken(access_token): &AccessToken,
    ) -> anyhow::Result<Octocrab> {
        prepare_octocrab_client(&self.github_api_base_url)
            .context("Invalid GitHub client configuration")?
            .user_access_token(access_token.as_str())
            .build()
            .context("Unable to build GitHub client")
    }

    /// Create a GitHub client authenticated as a user with the given authenticated Octocrab client.
    pub async fn get_user_client(
        &self,
        repo: GithubRepoName,
        authenticated_client: Octocrab,
    ) -> anyhow::Result<UserGitHubClient> {
        let user = authenticated_client
            .current()
            .user()
            .await
            .context("Cannot get user authenticated with OAuth")?;

        let client_repo = GithubRepoName::new(&user.login, repo.name());
        let client =
            GithubRepositoryClient::new(user.html_url.clone(), authenticated_client, client_repo);
        Ok(UserGitHubClient {
            user: user.into(),
            client,
        })
    }

    pub fn authorization_url(&self, redirect_uri: Option<String>) -> String {
        let mut url = format!(
            "{base_url}/login/oauth/authorize?client_id={client_id}&scope={scope}",
            base_url = self.github_base_url,
            client_id = self.config.client_id(),
            scope = "public_repo,workflow",
        );
        if let Some(redirect_uri) = redirect_uri {
            let state = BASE64_URL_SAFE.encode(redirect_uri);
            write!(&mut url, "&state={state}").unwrap();
        }
        url
    }
}

const GITHUB_SESSION_ID: &str = "github-session";

#[derive(serde::Deserialize, serde::Serialize)]
pub struct GitHubSession {
    pub access_token: AccessToken,
}

impl GitHubSession {
    pub fn save(session: &SessionNullSession, access_token: AccessToken) {
        session.set(GITHUB_SESSION_ID, GitHubSession { access_token });
    }

    pub fn restore(session: &SessionNullSession) -> Option<GitHubSession> {
        session.get(GITHUB_SESSION_ID)
    }
}

/// GitHub client authenticated to work with a user's fork of a repository managed by bors.
pub struct UserGitHubClient {
    pub user: GithubUser,
    pub client: GithubRepositoryClient,
}

#[derive(Clone)]
pub struct OAuthConfig {
    client_id: String,
    client_secret: SecretString,
}

impl OAuthConfig {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret: client_secret.into(),
        }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn client_secret(&self) -> &str {
        self.client_secret.expose_secret()
    }
}
