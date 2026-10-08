INSERT INTO workflow (
    build_id,
    name,
    url,
    run_id,
    platform,
    status
)
SELECT
    id,
    'my circleci workflow',
    'https://github.com/fake/repo/runs/010101011111',
    100000,
    'github',
    'pending'
FROM build
ORDER BY id DESC
LIMIT 1;
