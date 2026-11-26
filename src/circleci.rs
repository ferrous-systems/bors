use std::{ops::Deref, sync::Arc};

use http::{HeaderMap, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::utils::timing::{RetryMethod, perform_retryable};

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(rename_all = "kebab-case")]
pub struct CircleCiExternalGitHubCheckRunId {
    #[allow(unused)]
    pub actor_id: Option<String>,
    pub source: CircleCiGitHubCheckRunSource,
    pub workflow_id: CircleCiWorkflowId,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(rename_all = "kebab-case")]
pub enum CircleCiGitHubCheckRunSource {
    Notifications,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
#[serde(transparent)]
pub struct CircleCiWorkflowId(String);

impl CircleCiWorkflowId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CircleCiWorkflowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        String::fmt(&self.0, f)
    }
}

impl From<String> for CircleCiWorkflowId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<CircleCiWorkflowId> for String {
    fn from(value: CircleCiWorkflowId) -> Self {
        value.0
    }
}

impl Deref for CircleCiWorkflowId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct CircleCiClient(Arc<CircleCiClientInner>);

struct CircleCiClientInner {
    client: reqwest::Client,
    url: String,
}

impl CircleCiClient {
    pub fn new(url: String, token: SecretString) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        assert!(
            headers
                .insert(
                    "Authorization",
                    HeaderValue::from_str(&format!("Bearer {}", token.expose_secret())).unwrap(),
                )
                .is_none()
        );

        let client = reqwest::ClientBuilder::new()
            .user_agent("handlebors-ferrous-systems")
            .default_headers(headers)
            .build()?;

        Ok(Self(Arc::new(CircleCiClientInner { client, url })))
    }

    pub async fn cancel_workflow(&self, workflow: &CircleCiWorkflowId) -> anyhow::Result<()> {
        tracing::debug!("Cancelling CircleCI workflow {workflow}");
        let endpoint = format!("{}/api/v3/workflows/{workflow}/cancel", self.0.url);
        perform_retryable(
            "circleci_cancel_workflow",
            RetryMethod::default(),
            async || -> reqwest::Result<()> {
                // TODO: validate the result? CircleCI returns `{"data":{"id":"<workflow-id>"}}`
                let result = self.0.client.post(&endpoint).send().await?;
                match result.error_for_status_ref() {
                    Ok(_) => Ok(()),
                    Err(error) => {
                        let body: serde_json::Value = result.json().await?;
                        tracing::error!(%body, "CircleCI reported error");
                        Err(error)
                    }
                }
            },
        )
        .await?;
        Ok(())
    }
}
