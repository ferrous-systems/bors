CREATE TABLE IF NOT EXISTS check_run (
    id SERIAL PRIMARY KEY,
    build_id INT NOT NULL,
    check_run_id BIGINT NOT NULL, -- github check-run id
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    github_workflow_run_id BIGINT, -- github workflow-run id, null if not a github workflow run
    github_app_id BIGINT, -- the GitHub app id of the app that created this check-run
    CONSTRAINT fk_build_id FOREIGN KEY (build_id) REFERENCES build (id) ON DELETE CASCADE
);

DROP INDEX IF EXISTS workflow_build_id_url_idx;
DROP TABLE IF EXISTS workflow;
