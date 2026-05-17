CREATE TABLE IF NOT EXISTS import_sources (
  id UUID PRIMARY KEY,
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  period_branch_id UUID NOT NULL REFERENCES period_branches(id),
  label TEXT NOT NULL,
  file_name TEXT,
  file_hash TEXT NOT NULL,
  parser TEXT NOT NULL,
  row_count INT NOT NULL CHECK (row_count >= 0),
  uploaded_by_user_id TEXT NOT NULL,
  uploaded_by_name TEXT NOT NULL,
  uploaded_by_email TEXT NOT NULL,
  uploaded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (period_branch_id, file_hash)
);

ALTER TABLE trial_balance_lines
  ADD COLUMN IF NOT EXISTS source_id UUID REFERENCES import_sources(id);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'trial_balance_lines_period_account_unique'
  ) THEN
    ALTER TABLE trial_balance_lines
      ADD CONSTRAINT trial_balance_lines_period_account_unique
      UNIQUE (period_branch_id, account_id);
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'adjustments_period_reference_unique'
  ) THEN
    ALTER TABLE adjustments
      ADD CONSTRAINT adjustments_period_reference_unique
      UNIQUE (period_branch_id, reference);
  END IF;
END $$;

ALTER TABLE approvals
  ADD COLUMN IF NOT EXISTS commit_id UUID REFERENCES commits(id),
  ADD COLUMN IF NOT EXISTS snapshot_hash TEXT,
  ADD COLUMN IF NOT EXISTS approval_hash TEXT;

UPDATE approvals approval
SET commit_id = pack.commit_id
FROM review_packs pack
WHERE approval.review_pack_id = pack.id
  AND approval.commit_id IS NULL;

UPDATE approvals approval
SET snapshot_hash = commit.snapshot_hash
FROM commits commit
WHERE approval.commit_id = commit.id
  AND approval.snapshot_hash IS NULL;

UPDATE approvals
SET actor_user_id = COALESCE(NULLIF(actor_user_id, ''), 'legacy-unknown'),
    actor_email = COALESCE(NULLIF(actor_email, ''), 'legacy-unknown@example.invalid'),
    snapshot_hash = COALESCE(NULLIF(snapshot_hash, ''), 'legacy-unknown'),
    approval_hash = COALESCE(NULLIF(approval_hash, ''), md5(id::text || review_pack_id::text || role || actor_name))
WHERE actor_user_id IS NULL
   OR actor_email IS NULL
   OR snapshot_hash IS NULL
   OR approval_hash IS NULL;

ALTER TABLE approvals
  ALTER COLUMN actor_user_id SET NOT NULL,
  ALTER COLUMN actor_email SET NOT NULL,
  ALTER COLUMN commit_id SET NOT NULL,
  ALTER COLUMN snapshot_hash SET NOT NULL,
  ALTER COLUMN approval_hash SET NOT NULL;

ALTER TABLE approvals
  DROP CONSTRAINT IF EXISTS approvals_review_pack_id_role_key;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'approvals_pack_commit_role_unique'
  ) THEN
    ALTER TABLE approvals
      ADD CONSTRAINT approvals_pack_commit_role_unique
      UNIQUE (review_pack_id, commit_id, role);
  END IF;
END $$;

ALTER TABLE signed_pack_exports
  ADD COLUMN IF NOT EXISTS exported_by_user_id TEXT,
  ADD COLUMN IF NOT EXISTS exported_by_email TEXT;

UPDATE signed_pack_exports
SET exported_by_user_id = COALESCE(NULLIF(exported_by_user_id, ''), 'legacy-unknown'),
    exported_by_email = COALESCE(NULLIF(exported_by_email, ''), 'legacy-unknown@example.invalid')
WHERE exported_by_user_id IS NULL
   OR exported_by_email IS NULL;

ALTER TABLE signed_pack_exports
  ALTER COLUMN exported_by_user_id SET NOT NULL,
  ALTER COLUMN exported_by_email SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_signed_pack_exports_pack_hash
  ON signed_pack_exports(review_pack_id, payload_hash);

CREATE INDEX IF NOT EXISTS idx_import_sources_branch
  ON import_sources(period_branch_id, uploaded_at DESC);

CREATE OR REPLACE FUNCTION prevent_append_only_mutation()
RETURNS trigger AS $$
BEGIN
  RAISE EXCEPTION 'append-only table % does not allow %', TG_TABLE_NAME, TG_OP;
END;
$$ LANGUAGE plpgsql;

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'commits_append_only_guard') THEN
    CREATE TRIGGER commits_append_only_guard
      BEFORE UPDATE OR DELETE ON commits
      FOR EACH ROW EXECUTE FUNCTION prevent_append_only_mutation();
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'approvals_append_only_guard') THEN
    CREATE TRIGGER approvals_append_only_guard
      BEFORE UPDATE OR DELETE ON approvals
      FOR EACH ROW EXECUTE FUNCTION prevent_append_only_mutation();
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'audit_events_append_only_guard') THEN
    CREATE TRIGGER audit_events_append_only_guard
      BEFORE UPDATE OR DELETE ON audit_events
      FOR EACH ROW EXECUTE FUNCTION prevent_append_only_mutation();
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'signed_pack_exports_append_only_guard') THEN
    CREATE TRIGGER signed_pack_exports_append_only_guard
      BEFORE UPDATE OR DELETE ON signed_pack_exports
      FOR EACH ROW EXECUTE FUNCTION prevent_append_only_mutation();
  END IF;
END $$;
