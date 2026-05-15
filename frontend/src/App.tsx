import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import {
  approveReviewer,
  commitCorrection,
  getRepoWorkspace,
  listRepos,
  signClient,
} from "./api";
import {
  absoluteDecimal,
  branchStatusLabel,
  decimal,
  formatCurrency,
  formatDate,
  formatHash,
  formatSignedCurrency,
  reviewStatusLabel,
  roleLabel,
} from "./format";
import type {
  Commit,
  FinancialStatementLine,
  LegalEntityRepo,
  RepoWorkspace,
  ReviewStatus,
} from "./types";

export function App() {
  const [repos, setRepos] = useState<LegalEntityRepo[]>([]);
  const [selectedRepoId, setSelectedRepoId] = useState<string | null>(null);
  const [workspace, setWorkspace] = useState<RepoWorkspace | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionPending, setActionPending] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function loadInitial(isActive: () => boolean = () => true) {
    try {
      setLoading(true);
      setError(null);
      const repoList = await listRepos();
      if (!isActive()) return;

      setRepos(repoList);
      const firstRepoId = repoList[0]?.id ?? null;
      setSelectedRepoId(firstRepoId);

      const nextWorkspace = firstRepoId ? await getRepoWorkspace(firstRepoId) : null;
      if (!isActive()) return;
      setWorkspace(nextWorkspace);
    } catch (caught) {
      if (isActive()) {
        setWorkspace(null);
        setError(caught instanceof Error ? caught.message : "Failed to load repo");
      }
    } finally {
      if (isActive()) setLoading(false);
    }
  }

  useEffect(() => {
    let active = true;

    void loadInitial(() => active);

    return () => {
      active = false;
    };
  }, []);

  async function reloadWorkspace(repoId = selectedRepoId) {
    if (!repoId) return;
    const [nextWorkspace, nextRepos] = await Promise.all([getRepoWorkspace(repoId), listRepos()]);
    setWorkspace(nextWorkspace);
    setRepos(nextRepos);
  }

  async function runAction(label: string, action: () => Promise<void>) {
    try {
      setActionPending(label);
      setError(null);
      await action();
      await reloadWorkspace();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Action failed");
    } finally {
      setActionPending(null);
    }
  }

  async function handleRepoSelect(repoId: string) {
    if (repoId === selectedRepoId || actionPending !== null) return;

    try {
      setActionPending(`repo:${repoId}`);
      setError(null);
      const nextWorkspace = await getRepoWorkspace(repoId);
      setSelectedRepoId(repoId);
      setWorkspace(nextWorkspace);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Failed to load selected repo");
    } finally {
      setActionPending(null);
    }
  }

  if (loading) {
    return <LoadingScreen />;
  }

  if (!workspace) {
    return (
      <main className="empty-state">
        <p className="eyebrow">Accounts Repo</p>
        <h1>No financial repo is available.</h1>
        {error ? <p className="error-copy" role="alert">{error}</p> : null}
        {error ? (
          <button className="primary-button" onClick={() => void loadInitial()} type="button">
            Retry connection
          </button>
        ) : null}
      </main>
    );
  }

  const headCommit = workspace.commits.find((commit) => commit.id === workspace.branch.head_commit_id) ??
    workspace.commits[workspace.commits.length - 1];
  const firstCommit = workspace.commits[0];

  if (!headCommit || !firstCommit) {
    return (
      <main className="empty-state">
        <p className="eyebrow">Accounts Repo</p>
        <h1>This repo has no financial snapshots yet.</h1>
        <p className="empty-copy">Import a trial balance to create the first reviewable commit.</p>
      </main>
    );
  }

  const branchFrozen = workspace.branch.status === "frozen" || workspace.review_pack.status === "signed";

  return (
    <main className="shell" aria-busy={actionPending !== null}>
      <aside className="repo-rail" aria-label="Financial repositories">
        <div className="brand-block">
          <span className="brand-mark">AR</span>
          <div>
            <p className="eyebrow">Accounts Repo</p>
            <strong>Financial truth, versioned.</strong>
          </div>
        </div>

        <nav className="repo-list">
          {repos.map((repo) => {
            const isActive = repo.id === selectedRepoId;

            return (
              <button
                aria-current={isActive ? "page" : undefined}
                className={isActive ? "repo-button repo-button--active" : "repo-button"}
                disabled={actionPending !== null}
                key={repo.id}
                onClick={() => void handleRepoSelect(repo.id)}
                type="button"
              >
                <span>{repo.name}</span>
                <small>{repo.summary.active_branch_label}</small>
              </button>
            );
          })}
        </nav>

        <section className="collab-card">
          <p className="section-label">Custody</p>
          {workspace.repo.collaborators.map((collaborator) => (
            <div className="collab-row" key={collaborator.user_id}>
              <span>{collaborator.display_name}</span>
              <small>{roleLabel(collaborator.role)}</small>
            </div>
          ))}
        </section>
      </aside>

      <section className="workspace">
        <header className="hero-card">
          <div className="hero-copy">
            <p className="eyebrow">Client-owned legal entity repo</p>
            <h1>{workspace.repo.name}</h1>
            <p>
              Malaysia Sdn Bhd year-end branch with append-only commits, financial statement impact
              diff, reviewer approval, client sign-off, and immutable audit evidence.
            </p>
          </div>

          <div className="hero-ledger" aria-label="Current repo status">
            <StatusPill status={workspace.review_pack.status} />
            <div className="ledger-line">
              <span>Registration</span>
              <strong>{workspace.repo.registration_number}</strong>
            </div>
            <div className="ledger-line">
              <span>Head commit</span>
              <strong>{formatHash(headCommit.snapshot_hash)}</strong>
            </div>
            <div className="ledger-line">
              <span>Branch</span>
              <strong>{branchStatusLabel(workspace.branch.status)}</strong>
            </div>
          </div>
        </header>

        {error ? <div className="toast error-copy" role="alert">{error}</div> : null}

        <section className="metric-grid" aria-label="Financial summary">
          <MetricCard label="Revenue" value={formatCurrency(workspace.repo.summary.revenue)} tone="ink" />
          <MetricCard
            label="Profit before tax"
            value={formatCurrency(workspace.repo.summary.profit_before_tax)}
            tone={decimal(workspace.repo.summary.profit_before_tax) < 0 ? "bad" : "good"}
          />
          <MetricCard label="Net assets" value={formatCurrency(workspace.repo.summary.net_assets)} tone="gold" />
          <MetricCard label="Open commits" value={workspace.commits.length.toString()} tone="blue" />
        </section>

        <section className="flow-strip" aria-label="Year-end workflow">
          <FlowNode number="01" label="Intake" detail="TB export and evidence arrive raw" />
          <FlowNode number="02" label="Commit" detail="Preparer curates a financial snapshot" />
          <FlowNode number="03" label="Review pack" detail="Partner sees FS impact diff" />
          <FlowNode number="04" label="Sign-off" detail="Director freezes the branch" />
        </section>

        <div className="content-grid">
          <section className="panel panel--diff">
            <PanelHeader
              kicker="FS impact diff"
              title={`${formatHash(firstCommit.snapshot_hash)} -> ${formatHash(headCommit.snapshot_hash)}`}
              action={
                <button
                  className="ghost-button"
                  disabled={actionPending !== null || branchFrozen}
                  onClick={() =>
                    void runAction("correction", async () => {
                      await commitCorrection(workspace.repo.id, workspace.branch.id, {
                        actor_name: "Aina Rahman",
                        message: "Append correction for bank charge presentation",
                        reference: `AJ-${String(headCommit.snapshot.adjustments.length + 1).padStart(3, "0")}`,
                        description: "Reclass bank charges into administrative expenses",
                        rationale:
                          "Reviewer requested presentation under administrative expenses for Sdn Bhd accounts.",
                        lines: [
                          { account_code: "6000", amount: "3900.00" },
                          { account_code: "6400", amount: "-3900.00" },
                        ],
                      });
                    })
                  }
                  title={branchFrozen ? "Signed branches are immutable" : undefined}
                  type="button"
                >
                  {actionPending === "correction"
                    ? "Appending..."
                    : branchFrozen
                      ? "Branch frozen after sign-off"
                      : "Append correction commit"}
                </button>
              }
            />

            <div className="diff-headline">
              <DiffChip label="Revenue" value={workspace.fs_impact_diff.headline.revenue_change} />
              <DiffChip label="PBT" value={workspace.fs_impact_diff.headline.profit_before_tax_change} />
              <DiffChip label="Net assets" value={workspace.fs_impact_diff.headline.net_assets_change} />
            </div>

            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>FS line</th>
                    <th>Before</th>
                    <th>After</th>
                    <th>Change</th>
                  </tr>
                </thead>
                <tbody>
                  {workspace.fs_impact_diff.changed_fs_lines.map((line) => (
                    <tr key={line.fs_line}>
                      <td>{line.fs_line}</td>
                      <td>{formatCurrency(line.before, true)}</td>
                      <td>{formatCurrency(line.after, true)}</td>
                      <td className={decimal(line.change) < 0 ? "negative" : "positive"}>
                        {formatSignedCurrency(line.change)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>

          <ReviewPackPanel
            actionPending={actionPending}
            branchFrozen={branchFrozen}
            onApprove={() =>
              void runAction("approve", async () => {
                await approveReviewer(workspace.review_pack.id, {
                  actor_name: "Amjad Salleh",
                  note: "Reviewed TB mapping, adjustment rationale, and FS impact diff.",
                });
              })
            }
            onSign={() =>
              void runAction("sign", async () => {
                await signClient(workspace.review_pack.id, {
                  actor_name: "Hazli Johar",
                  note: "Director sign-off for the FY2026 year-end pack.",
                });
              })
            }
            pack={workspace.review_pack}
          />

          <section className="panel">
            <PanelHeader kicker="Commit chain" title="Append-only financial snapshots" />
            <div className="commit-list">
              {[...workspace.commits].reverse().map((commit) => (
                <CommitRow commit={commit} key={commit.id} />
              ))}
            </div>
          </section>

          <section className="panel">
            <PanelHeader kicker="Statement graph" title="Mapped FS lines" />
            <div className="fs-grid">
              {headCommit.snapshot.fs_lines.map((line) => (
                <FsLineCard line={line} key={line.fs_line} />
              ))}
            </div>
          </section>

          <section className="panel panel--wide">
            <PanelHeader kicker="Trial balance" title="Source accounts tied to the head commit" />
            <div className="table-wrap table-wrap--compact">
              <table>
                <thead>
                  <tr>
                    <th>Code</th>
                    <th>Account</th>
                    <th>Type</th>
                    <th>TB amount</th>
                  </tr>
                </thead>
                <tbody>
                  {headCommit.snapshot.trial_balance.map((line) => (
                    <tr key={line.account_code}>
                      <td className="mono">{line.account_code}</td>
                      <td>{line.account_name}</td>
                      <td>{roleLabel(line.account_type)}</td>
                      <td>{formatCurrency(line.amount, true)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>

          <section className="panel panel--wide">
            <PanelHeader kicker="Immutable audit trail" title="Every important event is preserved" />
            <div className="audit-list">
              {[...workspace.audit_events].reverse().map((event) => (
                <article className="audit-row" key={event.id}>
                  <span className="audit-dot" />
                  <div>
                    <strong>{roleLabel(event.event_type)}</strong>
                    <p>{event.message}</p>
                    <small>
                      {event.actor_name} · {formatDate(event.occurred_at)}
                    </small>
                  </div>
                </article>
              ))}
            </div>
          </section>
        </div>
      </section>
    </main>
  );
}

function LoadingScreen() {
  return (
    <main aria-busy="true" aria-live="polite" className="loading-screen" role="status">
      <div className="loading-orb" />
      <p className="eyebrow">Opening encrypted financial repo</p>
      <h1>Rebuilding authoritative snapshot...</h1>
    </main>
  );
}

function StatusPill({ status }: { status: ReviewStatus }) {
  const label = reviewStatusLabel(status);
  return (
    <span aria-label={`Review status: ${label}`} className={`status-pill status-pill--${status}`}>
      {label}
    </span>
  );
}

function MetricCard({ label, value, tone }: { label: string; value: string; tone: string }) {
  return (
    <article className={`metric-card metric-card--${tone}`}>
      <span>{label}</span>
      <strong>{value}</strong>
    </article>
  );
}

function FlowNode({ number, label, detail }: { number: string; label: string; detail: string }) {
  return (
    <article className="flow-node">
      <span>{number}</span>
      <strong>{label}</strong>
      <p>{detail}</p>
    </article>
  );
}

function PanelHeader({
  kicker,
  title,
  action,
}: {
  kicker: string;
  title: string;
  action?: ReactNode;
}) {
  return (
    <header className="panel-header">
      <div>
        <p className="section-label">{kicker}</p>
        <h2>{title}</h2>
      </div>
      {action}
    </header>
  );
}

function DiffChip({ label, value }: { label: string; value: string }) {
  const amount = decimal(value);
  return (
    <div className="diff-chip">
      <span>{label}</span>
      <strong className={amount < 0 ? "negative" : amount > 0 ? "positive" : "neutral"}>
        {formatSignedCurrency(value)}
      </strong>
    </div>
  );
}

function ReviewPackPanel({
  pack,
  actionPending,
  branchFrozen,
  onApprove,
  onSign,
}: {
  pack: RepoWorkspace["review_pack"];
  actionPending: string | null;
  branchFrozen: boolean;
  onApprove: () => void;
  onSign: () => void;
}) {
  const hasReviewerApproval = pack.approvals.some((approval) => approval.role === "reviewer");
  const hasClientSignature = pack.approvals.some((approval) => approval.role === "client_director");
  const openQueryLabel = `${pack.open_queries.length} open ${pack.open_queries.length === 1 ? "query" : "queries"}`;

  return (
    <section className="panel panel--review">
      <PanelHeader kicker="Review pack" title={pack.title} />
      <StatusPill status={pack.status} />

      <div className="approval-stack">
        <ApprovalStep complete={hasReviewerApproval} label="Reviewer approval" />
        <ApprovalStep complete={hasClientSignature} label="Client director sign-off" />
      </div>

      <div className="query-box">
        <span>{openQueryLabel}</span>
        {pack.open_queries.map((query) => (
          <p key={query.id}>{query.title}</p>
        ))}
      </div>

      <div className="approval-actions">
        <button
          className="primary-button"
          disabled={branchFrozen || pack.status !== "in_review" || actionPending !== null}
          onClick={onApprove}
          type="button"
        >
          {actionPending === "approve" ? "Approving..." : "Approve as reviewer"}
        </button>
        <button
          className="primary-button primary-button--dark"
          disabled={branchFrozen || pack.status !== "reviewer_approved" || actionPending !== null}
          onClick={onSign}
          type="button"
        >
          {actionPending === "sign" ? "Signing..." : "Sign as client"}
        </button>
      </div>
    </section>
  );
}

function ApprovalStep({ complete, label }: { complete: boolean; label: string }) {
  return (
    <div className={complete ? "approval-step approval-step--complete" : "approval-step"}>
      <span>{complete ? "Complete" : "Waiting"}</span>
      <strong>{label}</strong>
    </div>
  );
}

function CommitRow({ commit }: { commit: Commit }) {
  return (
    <article className="commit-row">
      <div className="commit-node">{commit.sequence_number}</div>
      <div>
        <strong>{commit.message}</strong>
        <p>
          {formatHash(commit.snapshot_hash)} · {commit.created_by} · {formatDate(commit.created_at)}
        </p>
      </div>
    </article>
  );
}

function FsLineCard({ line }: { line: FinancialStatementLine }) {
  const amount = decimal(line.amount);
  const accountCount = line.account_codes.length;

  return (
    <article className="fs-card">
      <span>{accountCount} accounts</span>
      <strong>{line.fs_line}</strong>
      <p className={amount < 0 ? "negative" : "positive"}>{formatCurrency(absoluteDecimal(line.amount), true)}</p>
    </article>
  );
}
