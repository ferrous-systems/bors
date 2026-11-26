use std::sync::Arc;

use anyhow::ensure;
use octocrab::models::CheckRunId;
use parking_lot::Mutex;
use wiremock::MockServer;
use wiremock::ResponseTemplate;

use crate::circleci::CircleCiClient;
use crate::circleci::CircleCiWorkflowId;
use crate::tests::GitHub;
use crate::tests::mock::dynamic_mock_req;

const TEST_CIRCLECI_TOKEN: &str = "test-circleci-token";

pub(super) struct CircleCiMockServer {
    mock_server: MockServer,
    workflows: Arc<Mutex<Vec<(CircleCiWorkflowId, CheckRunId)>>>,
    cancelled_workflows: Arc<Mutex<Vec<(CircleCiWorkflowId, CheckRunId)>>>,
}

impl CircleCiMockServer {
    pub(super) async fn start(github: Arc<Mutex<GitHub>>) -> Self {
        let workflows = Arc::<Mutex<Vec<(CircleCiWorkflowId, CheckRunId)>>>::default();
        let cancelled_workflows = Arc::<Mutex<Vec<(CircleCiWorkflowId, CheckRunId)>>>::default();

        let mock_server = MockServer::start().await;
        dynamic_mock_req(
            {
                let workflows = workflows.clone();
                let cancelled_workflows = cancelled_workflows.clone();
                move |_, [workflow_id]| {
                    let mut workflows = workflows.lock();
                    let Some(idx) = workflows
                        .iter()
                        .position(|(w, _)| workflow_id == w.as_str())
                    else {
                        return ResponseTemplate::new(404);
                    };
                    let workflow @ (_, check_run_id) = workflows.remove(idx);

                    let github = github.lock();
                    let repo = github.get_repo_by_check_run_id(check_run_id);
                    let repo = repo.lock();
                    if repo.workflow_cancel_error {
                        workflows.push(workflow);
                        return ResponseTemplate::new(500);
                    }

                    let mut cancelled_workflows = cancelled_workflows.lock();
                    cancelled_workflows.push(workflow);

                    ResponseTemplate::new(200)
                }
            },
            "POST",
            "^/api/v3/workflows/([^/]*)/cancel$".to_string(),
        )
        .mount(&mock_server)
        .await;

        Self {
            mock_server,
            workflows,
            cancelled_workflows,
        }
    }

    pub(super) fn client(&self) -> CircleCiClient {
        CircleCiClient::new(self.mock_server.uri(), TEST_CIRCLECI_TOKEN.into()).unwrap()
    }

    pub(super) fn add_workflow(&self, workflow_id: CircleCiWorkflowId, check_run_id: CheckRunId) {
        let mut workflows = self.workflows.lock();
        workflows.push((workflow_id, check_run_id));
    }

    pub(super) fn is_workflow_cancelled(&self, workflow_id: &CircleCiWorkflowId) -> bool {
        let mut cancelled = self.cancelled_workflows.lock();
        let Some(pos) = cancelled.iter().position(|(w, _)| w == workflow_id) else {
            return false;
        };
        cancelled.remove(pos);
        true
    }

    pub(super) async fn assert_empty_queues(self) -> anyhow::Result<()> {
        self.mock_server.reset().await;
        let cancelled = self.cancelled_workflows.lock();
        ensure!(
            cancelled.is_empty(),
            "Some cancelled CircleCI workflows weren't checked"
        );
        Ok(())
    }
}
