DELETE FROM workflow
WHERE platform != 'github';

ALTER TABLE IF EXISTS workflow
RENAME COLUMN platform to type;

ALTER TABLE IF EXISTS workflow
ALTER COLUMN run_id SET DATA TYPE BIGINT;
