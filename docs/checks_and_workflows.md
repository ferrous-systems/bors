# GitHub Check and Workflows

GitHub allows for commits to be tested using check-suites, which contain a series of check-runs. Check-runs are added to check-suites, and once all check-runs have completed, the check-suite's status is determined using the statuses of the child check-runs.

GitHub Actions extends this system somewhat by introducing workflows, workflow-runs, and workflow-jobs. Each workflow (.yml file in `.github/workflows`) is instantiated as a workflow-run, which is analagous to a check-suite. In fact, the data GitHub returns for a workflow-run includes a check-suite id that can be used to additionally reference the workflow-run.

Each job in a Workflow is then instantiated, and included as a workflow-job under the workflow-run, and additionally as a check-run under the corresponding check-suite. In fact, the id of the workflow-job is the exact same as the id for the check-run.

## CircleCI

CircleCI's checks integration is a little simpler - each check-suite has exactly one check-run, which is the workflow in a run (using their terminology according to the [API docs](https://circleci.com/docs/api/v3#tag/workflows)). Once the workflow is completed, CircleCI reports the status back to GitHub's check-run API, which then completes the check-suite.

CircleCI includes the UUID that identifies the workflow in the `external_id` field of the check-run (which doesn't have an equivalent in the check-suite API), which can be used in API calls to CircleCI for linking to and interacting with the workflow. The `external_id` is a JSON object that looks like:

```json
{"actor-id":"UUID","source":"notifications","workflow-id":"UUID"}
```

Currently, there's a check to only interact with `"source":"notifications"`, though other values may be used and handled later.

It's also not entirely clear what the `actor-id` is used for. This may be documented somewhere, but it's not necessary for Bors to use at the moment.
