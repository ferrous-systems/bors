use chrono::{DateTime, Utc};
use octocrab::models::{App, CheckRunId, CheckSuiteId};
use octocrab::params::checks::{CheckRunConclusion, CheckRunStatus};
use serde::Serialize;
use url::Url;

use crate::github::CIRCLECI_CHECKS_APP_ID;
use crate::tests::Repo;
use crate::tests::github::{CheckRunData, CheckRunEventKind};
use crate::tests::mock::repository::GitHubRepository;

#[derive(Serialize)]
pub struct GitHubCheckRunEventPayload {
    action: String,
    check_run: GitHubCheckRun,
    repository: GitHubRepository,
}

impl GitHubCheckRunEventPayload {
    pub fn new(repo: &Repo, run: CheckRunData, event: CheckRunEventKind) -> Self {
        let repository = GitHubRepository::from(repo);
        let action = match &event {
            CheckRunEventKind::Started => "created",
            CheckRunEventKind::Completed { .. } => "completed",
        };
        Self {
            repository,
            action: action.to_string(),
            check_run: GitHubCheckRun::new(repo, run),
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
    status: CheckRunStatus,
    conclusion: Option<CheckRunConclusion>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    output: GitHubCheckRunOutput,
    check_suite: GitHubCheckSuite,
    app: App,
    pull_requests: Vec<()>,
}

impl GitHubCheckRun {
    pub fn new(repo: &Repo, run: CheckRunData) -> Self {
        let make_url = |prefix: &str, rest: &str| -> Url {
            format!("https://{prefix}github.com/{}{rest}", repo.full_name())
                .parse()
                .unwrap()
        };

        let app: App = serde_json::from_value(serde_json::json!({
            "id": run.github_app_id,
            "node_id": "A_fake_node_id",
            "name": match run.github_app_id {
                CIRCLECI_CHECKS_APP_ID => "circleci-checks".to_string(),
                // GITHUB_ACTIONS_APP_ID => "github-actions".to_string(),
                _ => format!("app{}", run.github_app_id),
            },
            "external_url": match run.github_app_id {
                CIRCLECI_CHECKS_APP_ID => "https://circleci.com".to_string(),
                // GITHUB_ACTIONS_APP_ID => "https://help.github.com/en/actions".to_string(),
                _ => format!("https://example.com/app{}", run.github_app_id),
            },
            "html_url": match run.github_app_id {
                CIRCLECI_CHECKS_APP_ID => "https://github.com/apps/circleci-checks".to_string(),
                // GITHUB_ACTIONS_APP_ID => "https://github.com/apps/github-actions".to_string(),
                _ => format!("https://github.com/apps/app{}", run.github_app_id),
            },
            "permissions": {},
            "events": [],
            "owner": {
                "login": "Kobzol",
                "id": 4539057,
                "node_id": "U_fake_node_id",
                "type": "User",
                "site_admin": false,
                "node_id": "",
                "avatar_url": "https://api.github.com/users/Kobzol.png",
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

        GitHubCheckRun {
            id: run.id,
            name: run.name,
            node_id: "CR_fake_node_id".to_string(),
            head_sha: run.head_sha.clone(),
            external_id: run.external_id.clone(),
            url: make_url("api.", &format!("/check-runs/{}", run.id)),
            html_url: make_url("", &format!("/actions/runs/{}", run.id)),
            details_url: make_url("", &format!("/actions/runs/{}/details", run.id)), // very fake
            status: run.status.clone(),
            conclusion: run.conclusion,
            started_at: run.started_at,
            completed_at: run.completed_at,
            output: GitHubCheckRunOutput {
                title: run.title,
                summary: run.summary,
                text: None,
                annotations_count: 0,
                annotations_url: make_url("api.", &format!("/check-runs/{}/annotations", run.id)),
            },
            check_suite: GitHubCheckSuite {
                id: CheckSuiteId(0),
                node_id: "".to_string(),
                head_branch: run.head_branch.clone(),
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
        }
    }
}

#[derive(Serialize)]
pub struct GitHubCheckRunOutput {
    title: String,
    summary: String,
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
    status: CheckRunStatus,
    conclusion: Option<CheckRunConclusion>,
    url: Url,
    before: String,
    after: String,
    pull_requests: Vec<()>,
    app: App,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
