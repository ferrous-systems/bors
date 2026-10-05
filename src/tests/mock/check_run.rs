use chrono::{DateTime, Utc};
use octocrab::models::{App, CheckRunId, CheckSuiteId, workflows::Conclusion};
use serde::Serialize;
use url::Url;

use crate::github::GITHUB_ACTIONS_APP_ID;
use crate::tests::Repo;
use crate::tests::github::CheckRunData;
use crate::tests::mock::repository::GitHubRepository;

#[derive(Serialize)]
pub struct GitHubCheckRunEventPayload {
    action: String,
    check_run: GitHubCheckRun,
    repository: GitHubRepository,
}

impl GitHubCheckRunEventPayload {
    pub fn new(repo: &Repo, run: CheckRunData) -> Self {
        let make_url = |prefix: &str, rest: &str| -> Url {
            format!("https://{prefix}github.com/{}{rest}", repo.full_name())
                .parse()
                .unwrap()
        };

        let app: App = serde_json::from_value(serde_json::json!({
            "id": run.github_app_id,
            "name": match run.github_app_id {
                GITHUB_ACTIONS_APP_ID => "github-actions".to_string(),
                _ => "idk".to_string(),
            },
            "external_url": "https://example.com",
            "html_url": "https://example.com",
            "permissions": {},
            "events": [],
            "owner": {
                "login": "Kobzol",
                "id": 0,
                "node_id": "",
                "avatar_url": "",
                "gravatar_id": "",
                "url": "https://api.github.com/users/Kobzol",
                "html_url": "https://github.com/Kobzol",
                "followers_url": "https://api.github.com/Kobzol/followers",
                "following_url": "https://api.github.com/Kobzol/following",
                "gists_url": "https://api.github.com/Kobzol/gists",
                "starred_url": "https://api.github.com/Kobzol/starred",
                "subscriptions_url": "https://api.github.com/Kobzol/subscriptions",
                "organizations_url": "https://api.github.com/Kobzol/organizations",
                "repos_url": "https://api.github.com/Kobzol/repos",
                "events_url": "https://api.github.com/Kobzol/events",
                "received_events_url": "https://api.github.com/Kobzol/received_events",
            },
        }))
        .unwrap();

        Self {
            action: match run.status.as_str() {
                "queued" | "in_progress" => "created".to_string(),
                _ => "completed".to_string(),
            },
            repository: GitHubRepository::from(repo),
            check_run: GitHubCheckRun {
                id: run.id,
                name: run.name,
                node_id: "".to_string(),
                head_sha: run.head_sha.clone(),
                external_id: run.external_id,
                url: make_url("api.", &format!("/check-runs/{}", run.id)),
                html_url: make_url("", &format!("/actions/runs/{}/job/1010101", run.id)),
                details_url: make_url("", &format!("/actions/runs/{}/job/1010101", run.id)),
                status: run.status.clone(),
                conclusion: run.conclusion.clone(),
                started_at: run.started_at,
                completed_at: run.completed_at,
                output: GitHubCheckRunOutput {
                    title: (!run.title.is_empty()).then_some(run.title),
                    summary: (!run.summary.is_empty()).then_some(run.summary),
                    text: (!run.text.is_empty()).then_some(run.text),
                    annotations_count: 0,
                    annotations_url: format!("https://api.github.com/foo/bar").parse().unwrap(),
                },
                check_suite: GitHubCheckSuite {
                    id: CheckSuiteId(0),
                    node_id: "".to_string(),
                    head_branch: None,
                    head_sha: run.head_sha.clone(),
                    status: run.status,
                    conclusion: run.conclusion,
                    url: make_url("api.", "/check-suites/0"),
                    before: "".to_string(),
                    after: run.head_sha,
                    pull_requests: vec![],
                    app: app.clone(),
                    created_at: run.started_at,
                    updated_at: run.completed_at.unwrap_or(run.started_at),
                },
                app,
                pull_requests: vec![],
            },
        }
    }
}

#[derive(Serialize)]
pub struct GitHubCheckRun {
    id: CheckRunId,
    name: String,
    node_id: String,
    head_sha: String,
    external_id: String,
    url: Url,
    html_url: Url,
    details_url: Url,
    status: String,
    conclusion: Option<Conclusion>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    output: GitHubCheckRunOutput,
    check_suite: GitHubCheckSuite,
    app: App,
    pull_requests: Vec<()>,
}

#[derive(Serialize)]
pub struct GitHubCheckRunOutput {
    title: Option<String>,
    summary: Option<String>,
    text: Option<String>,
    annotations_count: u32,
    annotations_url: Url,
}

#[derive(Serialize)]
pub struct GitHubCheckSuite {
    id: CheckSuiteId,
    node_id: String,
    head_branch: Option<String>,
    head_sha: String,
    status: String,
    conclusion: Option<Conclusion>,
    url: Url,
    before: String,
    after: String,
    pull_requests: Vec<()>,
    app: App,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
