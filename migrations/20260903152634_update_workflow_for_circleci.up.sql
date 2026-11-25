ALTER TABLE IF EXISTS workflow
RENAME COLUMN type TO platform;

ALTER TABLE IF EXISTS workflow
ALTER COLUMN run_id SET DATA TYPE TEXT;

ALTER TABLE IF EXISTS build
ADD COLUMN expects_circleci_workflow BOOLEAN;

UPDATE build
SET expects_circleci_workflow = FALSE;

ALTER TABLE IF EXISTS build
ALTER COLUMN expects_circleci_workflow
SET NOT NULL;
