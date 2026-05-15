CREATE TABLE organizations (
  id UUID PRIMARY KEY,
  name TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE users (
  id UUID PRIMARY KEY,
  display_name TEXT NOT NULL,
  email TEXT NOT NULL UNIQUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE legal_entities (
  id UUID PRIMARY KEY,
  owner_organization_id UUID NOT NULL REFERENCES organizations(id),
  name TEXT NOT NULL,
  registration_number TEXT NOT NULL,
  jurisdiction TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE repo_collaborators (
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  user_id UUID NOT NULL REFERENCES users(id),
  role TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (legal_entity_id, user_id)
);

CREATE TABLE period_branches (
  id UUID PRIMARY KEY,
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  label TEXT NOT NULL,
  period_start DATE NOT NULL,
  period_end DATE NOT NULL,
  status TEXT NOT NULL,
  head_commit_id UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE accounts (
  id UUID PRIMARY KEY,
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  code TEXT NOT NULL,
  name TEXT NOT NULL,
  account_type TEXT NOT NULL,
  UNIQUE (legal_entity_id, code)
);

CREATE TABLE trial_balance_lines (
  id UUID PRIMARY KEY,
  period_branch_id UUID NOT NULL REFERENCES period_branches(id),
  account_id UUID NOT NULL REFERENCES accounts(id),
  amount NUMERIC(18, 2) NOT NULL,
  source_label TEXT NOT NULL
);

CREATE TABLE mappings (
  id UUID PRIMARY KEY,
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  account_code TEXT NOT NULL,
  fs_line TEXT NOT NULL,
  assertion TEXT NOT NULL,
  UNIQUE (legal_entity_id, account_code)
);

CREATE TABLE adjustments (
  id UUID PRIMARY KEY,
  period_branch_id UUID NOT NULL REFERENCES period_branches(id),
  reference TEXT NOT NULL,
  description TEXT NOT NULL,
  rationale TEXT NOT NULL,
  created_by TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE adjustment_lines (
  id UUID PRIMARY KEY,
  adjustment_id UUID NOT NULL REFERENCES adjustments(id),
  account_code TEXT NOT NULL,
  amount NUMERIC(18, 2) NOT NULL
);

CREATE TABLE commits (
  id UUID PRIMARY KEY,
  period_branch_id UUID NOT NULL REFERENCES period_branches(id),
  sequence_number INT NOT NULL,
  message TEXT NOT NULL,
  previous_hash TEXT,
  snapshot_hash TEXT NOT NULL,
  created_by TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (period_branch_id, sequence_number)
);

ALTER TABLE period_branches
  ADD CONSTRAINT period_branches_head_commit_fk
  FOREIGN KEY (head_commit_id) REFERENCES commits(id);

CREATE TABLE review_packs (
  id UUID PRIMARY KEY,
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  period_branch_id UUID NOT NULL REFERENCES period_branches(id),
  commit_id UUID NOT NULL REFERENCES commits(id),
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  created_by TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE approvals (
  id UUID PRIMARY KEY,
  review_pack_id UUID NOT NULL REFERENCES review_packs(id),
  role TEXT NOT NULL,
  actor_name TEXT NOT NULL,
  note TEXT,
  approved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (review_pack_id, role)
);

CREATE TABLE audit_events (
  id UUID PRIMARY KEY,
  legal_entity_id UUID NOT NULL REFERENCES legal_entities(id),
  actor_name TEXT NOT NULL,
  event_type TEXT NOT NULL,
  message TEXT NOT NULL,
  occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_period_branches_entity ON period_branches(legal_entity_id);
CREATE INDEX idx_commits_branch_sequence ON commits(period_branch_id, sequence_number DESC);
CREATE INDEX idx_audit_events_entity_time ON audit_events(legal_entity_id, occurred_at DESC);
