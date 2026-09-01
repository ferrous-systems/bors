use std::sync::Arc;
use std::time::Duration;

use octocrab::models::CheckRunId;

use crate::PgDbClient;
use crate::bors::BuildKind;
use crate::bors::RepositoryState;
use crate::bors::build_queue::BuildQueueSender;
use crate::bors::comment::CommentTag;
use crate::bors::comment::append_check_run_links_to_comment;
use crate::bors::event::CheckRunCompleted;
use crate::bors::event::CheckRunCreated;
use crate::database::BuildModel;
use crate::database::BuildStatus;
use crate::database::WorkflowStatus;
use crate::github::CommitSha;
use crate::github::GithubRepoName;

async fn get_build(
    db: &PgDbClient,
    check_run_id: CheckRunId,
    repo: &GithubRepoName,
    commit: &CommitSha,
) -> anyhow::Result<Option<BuildModel>> {
    let mut builds = db.find_builds_by_sha(repo, commit).await?;

    // there *shouldn't* be multiple builds associated with a commit_sha:
    // - each merge commit is only attempted once
    // - each try commit is only attempted once
    // - if someone tries to merge a try commit, the merge commit will be after
    //      the try commit
    // - if someone tries to try a merge commit, the try commit will be after the
    //      merge commit
    // this should never happen, but it's worth checking for.
    // there shouldn't be any harm in handling this case anyways, but i'd rather
    // keep this assumption than handle an extremely unlikely case.
    if builds.len() > 1 {
        tracing::error!("Found multiple builds for commit, ignoring");
        return Ok(None);
    }

    let Some(build) = builds.pop() else {
        // instead of calling `is_bors_observed_branch`, which is what the
        // workflow_run handler does, ignore the check_run if it's not a tracked
        // build. this should be more reliable, since the head_branch field
        // isn't guaranteed for check runs/suites
        tracing::info!("Commit not tracked, ignoring");
        return Ok(None);
    };

    if build
        .check_run_id
        .is_some_and(|build_check_run_id| check_run_id == CheckRunId(build_check_run_id as _))
    {
        tracing::info!("Skipping bors-reported check-run");
        return Ok(None);
    }

    Ok(Some(build))
}

pub async fn handle_check_run_created(
    repo: Arc<RepositoryState>,
    db: Arc<PgDbClient>,
    payload: CheckRunCreated,
) -> anyhow::Result<()> {
    tracing::info!("Handling check run created");

    let Some(build) = get_build(&db, payload.id, &payload.repository, &payload.commit_sha).await?
    else {
        return Ok(());
    };

    // This can happen e.g. if the build is cancelled quickly
    if build.status != BuildStatus::Pending {
        tracing::warn!("Received check run created for an already completed build");
        return Ok(());
    }

    tracing::info!("Storing check run created into DB");
    db.create_check_run(
        payload.id,
        &payload.name,
        &build,
        &payload.html_url,
        payload.started_at,
    )
    .await?;

    add_check_run_links_to_build_start_comment(&repo, &db, &build, payload).await?;

    Ok(())
}

async fn add_check_run_links_to_build_start_comment(
    repo: &RepositoryState,
    db: &PgDbClient,
    build: &BuildModel,
    payload: CheckRunCreated,
) -> anyhow::Result<()> {
    let Some(pr) = db.find_pr_by_build(build).await? else {
        tracing::warn!("PR for build not found");
        return Ok(());
    };

    let tag = match build.kind {
        BuildKind::Try => CommentTag::TryBuildStarted,
        BuildKind::Auto => CommentTag::AutoBuildStarted,
    };
    let comments = db
        .get_tagged_bot_comments(&payload.repository, pr.number, tag)
        .await?;

    let Some(build_started_comment) = comments.last() else {
        tracing::warn!("No build started comment found for PR");
        return Ok(());
    };

    let check_runs = db.get_check_runs_for_build(build).await?;
    if !check_runs.is_empty() {
        let mut comment_content = repo
            .client
            .get_comment_content(&build_started_comment.node_id)
            .await?;

        comment_content = comment_content
            .lines()
            .take_while(|line| !line.starts_with("**Check"))
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
            .join("\n\n");

        append_check_run_links_to_comment(&mut comment_content, check_runs);

        repo.client
            .update_comment_content(&build_started_comment.node_id, &comment_content)
            .await?;
    }

    Ok(())
}

pub async fn handle_check_run_completed(
    repo: Arc<RepositoryState>,
    db: Arc<PgDbClient>,
    mut payload: CheckRunCompleted,
    build_queue_tx: &BuildQueueSender,
) -> anyhow::Result<()> {
    tracing::info!("Handling check run completed");

    let Some(build) = get_build(&db, payload.id, &payload.repository, &payload.commit_sha).await?
    else {
        return Ok(());
    };

    let mut error_context = None;
    if payload.status == WorkflowStatus::Success {
        let running_time = payload
            .running_time
            .map(|d| d.to_std().ok())
            .flatten()
            .unwrap_or(Duration::from_secs(0));
        if let Some(min_ci_time) = repo.config.load().min_ci_time
            && running_time < min_ci_time
        {
            tracing::warn!(
                minimum = min_ci_time.as_secs_f64(),
                elapsed = running_time.as_secs_f64(),
                "Check suite running time is less than the minimum CI duration",
            );
            payload.status = WorkflowStatus::Failure;
            error_context = Some(format!(
                "Check `{}` was considered to be a failure because it took only `{}s`. The minimum duration for CI check suites is configured to be `{}s`.",
                payload.name,
                running_time.as_secs_f64(),
                min_ci_time.as_secs_f64(),
            ));
        }
    }

    tracing::info!("Updating status of check run to {:?}", payload.status);
    db.update_check_run_status(payload.id, payload.status)
        .await?;

    build_queue_tx
        .on_check_run_completed(payload, build.branch, error_context)
        .await?;

    Ok(())
}
