CREATE TABLE IF NOT EXISTS organizations (
  id UUID PRIMARY KEY,
  name TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY,
  auth_user_id TEXT UNIQUE,
  display_name TEXT NOT NULL,
  email TEXT NOT NULL UNIQUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS legal_entities (
  id UUID PRIMARY KEY,
  owner_organization_id UUID NOT NULL REFERENCES organizations(id),
  name TEXT NOT NULL,
  registration_number TEXT NOT NULL,
  jurisdiction TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS repo_collaborators (
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  user_id UUID NOT NULL REFERENCES users(id),
  role TEXT NOT NULL CHECK (role IN ('owner', 'preparer', 'reviewer', 'client_signer', 'observer')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (legal_entity_id, user_id)
);

CREATE TABLE IF NOT EXISTS period_branches (
  id UUID PRIMARY KEY,
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  label TEXT NOT NULL,
  period_start DATE NOT NULL,
  period_end DATE NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('working', 'in_review', 'frozen')),
  head_commit_id UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (period_start <= period_end),
  UNIQUE (legal_entity_id, label)
);

CREATE TABLE IF NOT EXISTS accounts (
  id UUID PRIMARY KEY,
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  code TEXT NOT NULL,
  name TEXT NOT NULL,
  account_type TEXT NOT NULL CHECK (account_type IN ('asset', 'liability', 'equity', 'income', 'expense')),
  UNIQUE (legal_entity_id, code)
);

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

CREATE TABLE IF NOT EXISTS trial_balance_lines (
  id UUID PRIMARY KEY,
  period_branch_id UUID NOT NULL REFERENCES period_branches(id),
  account_id UUID NOT NULL REFERENCES accounts(id),
  amount NUMERIC(18, 2) NOT NULL,
  source_label TEXT NOT NULL,
  source_id UUID REFERENCES import_sources(id),
  CONSTRAINT trial_balance_lines_period_account_unique UNIQUE (period_branch_id, account_id)
);

CREATE TABLE IF NOT EXISTS mappings (
  id UUID PRIMARY KEY,
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  account_code TEXT NOT NULL,
  fs_line TEXT NOT NULL,
  assertion TEXT NOT NULL,
  UNIQUE (legal_entity_id, account_code)
);

CREATE TABLE IF NOT EXISTS adjustments (
  id UUID PRIMARY KEY,
  period_branch_id UUID NOT NULL REFERENCES period_branches(id),
  reference TEXT NOT NULL,
  description TEXT NOT NULL,
  rationale TEXT NOT NULL,
  created_by TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT adjustments_period_reference_unique UNIQUE (period_branch_id, reference)
);

CREATE TABLE IF NOT EXISTS adjustment_lines (
  id UUID PRIMARY KEY,
  adjustment_id UUID NOT NULL REFERENCES adjustments(id),
  account_code TEXT NOT NULL,
  amount NUMERIC(18, 2) NOT NULL
);

CREATE TABLE IF NOT EXISTS commits (
  id UUID PRIMARY KEY,
  period_branch_id UUID NOT NULL REFERENCES period_branches(id),
  sequence_number INT NOT NULL,
  message TEXT NOT NULL,
  previous_hash TEXT,
  snapshot_hash TEXT NOT NULL,
  snapshot_json JSONB NOT NULL,
  created_by TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (period_branch_id, sequence_number)
);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'period_branches_head_commit_fk'
  ) THEN
    ALTER TABLE period_branches
      ADD CONSTRAINT period_branches_head_commit_fk
      FOREIGN KEY (head_commit_id) REFERENCES commits(id);
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS review_packs (
  id UUID PRIMARY KEY,
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  period_branch_id UUID NOT NULL REFERENCES period_branches(id),
  commit_id UUID NOT NULL REFERENCES commits(id),
  title TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('in_review', 'reviewer_approved', 'signed')),
  created_by TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS approvals (
  id UUID PRIMARY KEY,
  review_pack_id UUID NOT NULL REFERENCES review_packs(id),
  commit_id UUID NOT NULL REFERENCES commits(id),
  role TEXT NOT NULL CHECK (role IN ('reviewer', 'client_director')),
  actor_user_id TEXT NOT NULL,
  actor_name TEXT NOT NULL,
  actor_email TEXT NOT NULL,
  snapshot_hash TEXT NOT NULL,
  approval_hash TEXT NOT NULL,
  note TEXT,
  approved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT approvals_pack_commit_role_unique UNIQUE (review_pack_id, commit_id, role)
);

CREATE TABLE IF NOT EXISTS review_queries (
  id UUID PRIMARY KEY,
  review_pack_id UUID NOT NULL REFERENCES review_packs(id),
  title TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('open', 'resolved')),
  assigned_to TEXT NOT NULL,
  resolved_note TEXT,
  resolved_by TEXT,
  resolved_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS audit_events (
  id UUID PRIMARY KEY,
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  sequence_number BIGINT NOT NULL,
  actor_user_id TEXT,
  actor_name TEXT NOT NULL,
  actor_email TEXT NOT NULL,
  event_type TEXT NOT NULL,
  message TEXT NOT NULL,
  occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  related_commit_id UUID REFERENCES commits(id),
  previous_hash TEXT,
  event_hash TEXT NOT NULL,
  UNIQUE (legal_entity_id, sequence_number),
  UNIQUE (legal_entity_id, event_hash)
);

CREATE TABLE IF NOT EXISTS signed_pack_exports (
  id UUID PRIMARY KEY,
  review_pack_id UUID NOT NULL REFERENCES review_packs(id),
  commit_id UUID NOT NULL REFERENCES commits(id),
  payload_json JSONB NOT NULL,
  payload_hash TEXT NOT NULL,
  exported_by TEXT NOT NULL,
  exported_by_user_id TEXT NOT NULL,
  exported_by_email TEXT NOT NULL,
  exported_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (review_pack_id, payload_hash)
);

CREATE TABLE IF NOT EXISTS app_state_snapshots (
  key TEXT PRIMARY KEY,
  payload JSONB NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_period_branches_entity ON period_branches(legal_entity_id);
CREATE INDEX IF NOT EXISTS idx_import_sources_branch ON import_sources(period_branch_id, uploaded_at DESC);
CREATE INDEX IF NOT EXISTS idx_commits_branch_sequence ON commits(period_branch_id, sequence_number DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_entity_time ON audit_events(legal_entity_id, occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_review_queries_pack_status ON review_queries(review_pack_id, status);
