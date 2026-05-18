import { useState } from "react";
import type { FormEvent, ReactNode } from "react";

import { decimal, formatCurrency, formatDate, formatHash, formatSignedCurrency, roleLabel } from "../format";
import { SheetFrame, StatusPill } from "../ui/primitives";
import type {
  Commit,
  CorrectionCommitPayload,
  FinancialStatementLine,
  ImportSource,
  RepoRole,
  RepoWorkspace,
  ReviewQuery,
  TrialBalanceLine,
} from "../types";
import { currentUserRoles, hasAnyRole, reviewActionMessage } from "./helpers";

export function ReviewWorkspace({
  actionPending,
  branchFrozen,
  currentUser,
  firstCommit,
  headCommit,
  onApprove,
  onCommitCorrection,
  onExportSignedPack,
  onOpenQuery,
  onResolveQuery,
  onSign,
  workspace,
}: {
  actionPending: string | null;
  branchFrozen: boolean;
  currentUser: { email: string };
  firstCommit: Commit;
  headCommit: Commit;
  onApprove: (note: string) => void;
  onCommitCorrection: (payload: CorrectionCommitPayload) => void;
  onExportSignedPack: () => void;
  onOpenQuery: (title: string) => void;
  onResolveQuery: (query: ReviewQuery) => void;
  onSign: (note: string) => void;
  workspace: RepoWorkspace;
}) {
  const [correctionOpen, setCorrectionOpen] = useState(false);
  const roles = currentUserRoles(workspace, currentUser.email);
  const canCommitCorrection = hasAnyRole(roles, ["preparer", "owner"]);

  return (
    <div className="review-layout">
      <section className="review-main">
        <SummaryStrip workspace={workspace} headCommit={headCommit} />
        <Panel
          action={
            branchFrozen ? (
              <span className="action-note">Corrections closed</span>
            ) : !canCommitCorrection ? (
              <span className="action-note">Preparer only</span>
            ) : (
              <button
                className="secondary-button"
                disabled={actionPending !== null}
                onClick={() => setCorrectionOpen(true)}
                type="button"
              >
                Append correction
              </button>
            )
          }
          meta={`${formatHash(firstCommit.snapshot_hash)} -> ${formatHash(headCommit.snapshot_hash)}`}
          title="Financial statement diff"
        >
          <div className="diff-headline">
            <DiffChip label="Revenue" value={workspace.fs_impact_diff.headline.revenue_change} />
            <DiffChip label="PBT" value={workspace.fs_impact_diff.headline.profit_before_tax_change} />
            <DiffChip label="Net assets" value={workspace.fs_impact_diff.headline.net_assets_change} />
          </div>

          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th scope="col">FS line</th>
                  <th className="numeric" scope="col">Before</th>
                  <th className="numeric" scope="col">After</th>
                  <th className="numeric" scope="col">Change</th>
                </tr>
              </thead>
              <tbody>
                {workspace.fs_impact_diff.changed_fs_lines.length === 0 ? (
                  <tr>
                    <td colSpan={4}>No FS line changes in this comparison.</td>
                  </tr>
                ) : (
                  workspace.fs_impact_diff.changed_fs_lines.map((line) => (
                    <tr key={line.fs_line}>
                      <td>{line.fs_line}</td>
                      <td className="numeric">{formatCurrency(line.before, true)}</td>
                      <td className="numeric">{formatCurrency(line.after, true)}</td>
                      <td className={`numeric ${decimal(line.change) < 0 ? "negative" : "positive"}`}>
                        {formatSignedCurrency(line.change)}
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
          <p className="table-hint">Swipe sideways to review before, after, and change columns.</p>
        </Panel>

        <CorrectionCommitSheet
          actionPending={actionPending}
          adjustmentsCount={headCommit.snapshot.adjustments.length}
          onOpenChange={setCorrectionOpen}
          onSubmit={(payload) => {
            onCommitCorrection(payload);
            setCorrectionOpen(false);
          }}
          open={correctionOpen && !branchFrozen && canCommitCorrection}
          trialBalance={headCommit.snapshot.trial_balance}
          workspace={workspace}
        />
      </section>

      <ReviewPackPanel
        actionPending={actionPending}
        branchFrozen={branchFrozen}
        currentUserRoles={roles}
        onApprove={onApprove}
        onExportSignedPack={onExportSignedPack}
        onOpenQuery={onOpenQuery}
        onResolveQuery={onResolveQuery}
        onSign={onSign}
        pack={workspace.review_pack}
        snapshotHash={headCommit.snapshot_hash}
      />
    </div>
  );
}

function CorrectionCommitSheet({
  actionPending,
  adjustmentsCount,
  onOpenChange,
  onSubmit,
  open,
  trialBalance,
  workspace,
}: {
  actionPending: string | null;
  adjustmentsCount: number;
  onOpenChange: (open: boolean) => void;
  onSubmit: (payload: CorrectionCommitPayload) => void;
  open: boolean;
  trialBalance: TrialBalanceLine[];
  workspace: RepoWorkspace;
}) {
  return (
    <SheetFrame description="Append a balanced correction commit to this review branch." onOpenChange={onOpenChange} open={open} title="Correction commit">
      <header className="sheet-header">
        <div>
          <h2>Correction commit</h2>
          <p>{workspace.repo.name} - {workspace.branch.label}</p>
        </div>
        <button className="nav-icon-button" onClick={() => onOpenChange(false)} type="button">Close</button>
      </header>
      <div className="sheet-body">
        <div className="settings-callout">
          <strong>Append-only financial source control</strong>
          <p>Corrections are committed as balanced adjustment lines and become part of the auditable chain.</p>
        </div>
        <CorrectionCommitForm
          actionPending={actionPending}
          adjustmentsCount={adjustmentsCount}
          onSubmit={onSubmit}
          trialBalance={trialBalance}
        />
      </div>
    </SheetFrame>
  );
}

function CorrectionCommitForm({
  actionPending,
  adjustmentsCount,
  onSubmit,
  trialBalance,
}: {
  actionPending: string | null;
  adjustmentsCount: number;
  onSubmit: (payload: CorrectionCommitPayload) => void;
  trialBalance: TrialBalanceLine[];
}) {
  const [message, setMessage] = useState("Append correction");
  const [reference, setReference] = useState(`AJ-${String(adjustmentsCount + 1).padStart(3, "0")}`);
  const [description, setDescription] = useState("");
  const [rationale, setRationale] = useState("");
  const [debitCode, setDebitCode] = useState("");
  const [debitAmount, setDebitAmount] = useState("");
  const [creditCode, setCreditCode] = useState("");
  const [creditAmount, setCreditAmount] = useState("");
  const runningBalance = decimal(debitAmount) - decimal(creditAmount);
  const isBalanced = debitAmount.trim() !== "" && creditAmount.trim() !== "" && runningBalance === 0;

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onSubmit({
      message,
      reference,
      description,
      rationale,
      lines: [
        { account_code: debitCode, amount: decimal(debitAmount).toFixed(2) },
        { account_code: creditCode, amount: (-decimal(creditAmount)).toFixed(2) },
      ],
    });
  }

  return (
    <form className="correction-form" onSubmit={handleSubmit}>
      <header className="panel-header">
        <div>
          <h2>Correction commit</h2>
          <p>Enter real adjustment lines. The backend rejects unbalanced entries.</p>
        </div>
      </header>

      <div className="form-grid">
        <label>
          Reference
          <input required value={reference} onChange={(event) => setReference(event.target.value)} />
        </label>
      </div>

      <label>
        Commit message
        <input required value={message} onChange={(event) => setMessage(event.target.value)} />
      </label>
      <label>
        Description
        <input required value={description} onChange={(event) => setDescription(event.target.value)} />
      </label>
      <label>
        Rationale
        <textarea required rows={3} value={rationale} onChange={(event) => setRationale(event.target.value)} />
      </label>

      <datalist id="trial-balance-accounts">
        {trialBalance.map((line) => (
          <option key={line.account_code} value={line.account_code}>
            {line.account_name}
          </option>
        ))}
      </datalist>

      <div className="form-grid">
        <label>
          Debit account code
          <input list="trial-balance-accounts" required value={debitCode} onChange={(event) => setDebitCode(event.target.value)} />
        </label>
        <label>
          Debit amount
          <input inputMode="decimal" required value={debitAmount} onChange={(event) => setDebitAmount(event.target.value)} />
        </label>
        <label>
          Credit account code
          <input list="trial-balance-accounts" required value={creditCode} onChange={(event) => setCreditCode(event.target.value)} />
        </label>
        <label>
          Credit amount
          <input inputMode="decimal" required value={creditAmount} onChange={(event) => setCreditAmount(event.target.value)} />
        </label>
      </div>

      <p className={isBalanced ? "balance-note balance-note--ok" : "balance-note"}>
        Running balance: {formatSignedCurrency(runningBalance.toFixed(2))}. Corrections must net to zero.
      </p>

      <button className="primary-button" disabled={actionPending !== null || !isBalanced} type="submit">
        {actionPending === "correction" ? "Appending..." : "Commit correction"}
      </button>
    </form>
  );
}

function SummaryStrip({ headCommit, workspace }: { headCommit: Commit; workspace: RepoWorkspace }) {
  return (
    <section className="summary-strip" aria-label="Financial summary">
      <SummaryItem label="Revenue" value={formatCurrency(workspace.repo.summary.revenue)} />
      <SummaryItem
        label="Profit before tax"
        tone={decimal(workspace.repo.summary.profit_before_tax) < 0 ? "bad" : "good"}
        value={formatCurrency(workspace.repo.summary.profit_before_tax)}
      />
      <SummaryItem label="Net assets" value={formatCurrency(workspace.repo.summary.net_assets)} />
      <SummaryItem label="Head commit" mono value={formatHash(headCommit.snapshot_hash)} />
    </section>
  );
}

function Panel({
  action,
  children,
  meta,
  title,
}: {
  action?: ReactNode;
  children: ReactNode;
  meta?: string;
  title: string;
}) {
  return (
    <section className="panel">
      <header className="panel-header">
        <div>
          <h2>{title}</h2>
          {meta ? <p>{meta}</p> : null}
        </div>
        {action}
      </header>
      {children}
    </section>
  );
}

function ReviewPackPanel({
  pack,
  actionPending,
  branchFrozen,
  currentUserRoles,
  onApprove,
  onExportSignedPack,
  onOpenQuery,
  onResolveQuery,
  onSign,
  snapshotHash,
}: {
  pack: RepoWorkspace["review_pack"];
  actionPending: string | null;
  branchFrozen: boolean;
  currentUserRoles: RepoRole[];
  onApprove: (note: string) => void;
  onExportSignedPack: () => void;
  onOpenQuery: (title: string) => void;
  onResolveQuery: (query: ReviewQuery) => void;
  onSign: (note: string) => void;
  snapshotHash: string;
}) {
  const [queryTitle, setQueryTitle] = useState("");
  const [approvalNote, setApprovalNote] = useState("Reviewed TB mapping, adjustment rationale, and FS impact diff.");
  const [signNote, setSignNote] = useState("Director sign-off for the current review pack snapshot.");
  const hasReviewerApproval = pack.approvals.some((approval) => approval.role === "reviewer");
  const hasClientSignature = pack.approvals.some((approval) => approval.role === "client_director");
  const openQueries = pack.open_queries.filter((query) => query.status === "open");
  const hasOpenQueries = openQueries.length > 0;
  const canOpenQuery = hasAnyRole(currentUserRoles, ["preparer", "reviewer", "owner"]);
  const canApprove = hasAnyRole(currentUserRoles, ["reviewer"]) && !hasAnyRole(currentUserRoles, ["owner", "preparer", "client_signer"]);
  const canSign = hasAnyRole(currentUserRoles, ["client_signer", "owner"]) && !hasAnyRole(currentUserRoles, ["preparer", "reviewer"]);
  const canExportSignedPack = hasAnyRole(currentUserRoles, ["owner", "client_signer", "reviewer"]);
  const querySummary = hasOpenQueries
    ? `${openQueries.length} open ${openQueries.length === 1 ? "query" : "queries"}`
    : pack.open_queries.length > 0
      ? "All queries resolved"
      : "No open queries";
  const nextAction = !branchFrozen && pack.status === "in_review" && canApprove
    ? {
        label: actionPending === "approve" ? "Approving..." : "Approve as reviewer",
        note: approvalNote,
        onChange: setApprovalNote,
        onSubmit: onApprove,
      }
    : !branchFrozen && pack.status === "reviewer_approved" && canSign
      ? {
          label: actionPending === "sign" ? "Signing..." : "Sign as client",
          note: signNote,
          onChange: setSignNote,
          onSubmit: onSign,
        }
      : null;

  function handleQuerySubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const title = queryTitle.trim();
    if (!title) return;
    onOpenQuery(title);
    setQueryTitle("");
  }

  return (
    <aside className="review-card" aria-label="Review pack">
      <div className="review-card__head">
        <p className="section-label">Review pack</p>
        <h2>{pack.title}</h2>
        <StatusPill status={pack.status} />
      </div>

      {!branchFrozen && canOpenQuery ? (
        <form className="query-composer" onSubmit={handleQuerySubmit}>
          <label htmlFor="review-query-title">Review query</label>
          <div className="query-composer__row">
            <input
              disabled={actionPending !== null}
              id="review-query-title"
              onChange={(event) => setQueryTitle(event.target.value)}
              placeholder="What must be resolved before approval?"
              value={queryTitle}
            />
            <button className="secondary-button" disabled={actionPending !== null || !queryTitle.trim()} type="submit">
              {actionPending === "open-query" ? "Opening..." : "Open query"}
            </button>
          </div>
        </form>
      ) : null}

      <div className="approval-stack">
        <ApprovalStep complete={hasReviewerApproval} label="Reviewer approval" />
        <ApprovalStep complete={hasClientSignature} label="Client director sign-off" />
      </div>

      {pack.open_queries.length > 0 ? (
        <section className="query-box" aria-label="Review queries">
          <header className="query-box__head">
            <strong>{querySummary}</strong>
            <span>{pack.open_queries.length} total</span>
          </header>
          {pack.open_queries.map((query) => (
            <div className="query-row" key={query.id}>
              <div>
                <p>{query.title}</p>
                <small>
                  Assigned to {query.assigned_to} - {query.status === "open" ? "Open" : "Resolved"}
                </small>
              </div>
              {query.status === "open" ? (
                <button className="secondary-button" disabled={actionPending !== null} onClick={() => onResolveQuery(query)} type="button">
                  {actionPending === `resolve:${query.id}` ? "Resolving..." : "Resolve query"}
                </button>
              ) : null}
            </div>
          ))}
        </section>
      ) : (
        <p className="quiet-note">No open queries.</p>
      )}

      {hasOpenQueries ? (
        <p className="action-note">Resolve open queries before approval or sign-off.</p>
      ) : null}

      {nextAction ? (
        <form
          className="approval-form"
          onSubmit={(event) => {
            event.preventDefault();
            nextAction.onSubmit(nextAction.note.trim());
          }}
        >
          <label>
            Evidence note
            <textarea
              disabled={actionPending !== null || hasOpenQueries}
              onChange={(event) => nextAction.onChange(event.target.value)}
              rows={3}
              value={nextAction.note}
            />
          </label>
          <p className="action-note">Records your verified identity against commit {formatHash(snapshotHash)}.</p>
          <button className="primary-button" disabled={actionPending !== null || hasOpenQueries || !nextAction.note.trim()} type="submit">
            {nextAction.label}
          </button>
        </form>
      ) : (
        <p className="action-note">{reviewActionMessage({ branchFrozen, canApprove, canSign, status: pack.status })}</p>
      )}

      {branchFrozen && canExportSignedPack ? (
        <button className="secondary-button" disabled={actionPending !== null} onClick={onExportSignedPack} type="button">
          Download signed export
        </button>
      ) : null}
    </aside>
  );
}

export function CommitPanel({ commits }: { commits: Commit[] }) {
  return (
    <Panel meta={`${commits.length} append-only snapshots`} title="Commit history">
      <div className="commit-list">
        {commits.length === 0 ? <p className="quiet-note">No commits yet.</p> : null}
        {[...commits].reverse().map((commit) => <CommitRow commit={commit} key={commit.id} />)}
      </div>
    </Panel>
  );
}

export function StatementsPanel({
  commit,
  importSources,
  lines,
}: {
  commit: Commit;
  importSources: ImportSource[];
  lines: FinancialStatementLine[];
}) {
  return (
    <Panel meta={`${lines.length} mapped lines`} title="Mapped financial statements">
      <div className="fs-grid">
        {lines.length === 0 ? <p className="quiet-note">No mapped FS lines yet.</p> : null}
        {lines.map((line) => (
          <FsLineCard commit={commit} importSources={importSources} line={line} key={line.fs_line} />
        ))}
      </div>
    </Panel>
  );
}

export function TrialBalancePanel({ commit, importSources }: { commit: Commit; importSources: ImportSource[] }) {
  const sourceById = new Map(importSources.map((source) => [source.id, source]));

  return (
    <Panel meta={`${commit.snapshot.trial_balance.length} source accounts`} title="Trial balance">
      <div className="table-wrap table-wrap--compact">
        <table>
          <thead>
            <tr>
              <th scope="col">Code</th>
              <th scope="col">Account</th>
              <th scope="col">Type</th>
              <th scope="col">Source</th>
              <th className="numeric" scope="col">TB amount</th>
            </tr>
          </thead>
          <tbody>
            {commit.snapshot.trial_balance.map((line) => {
              const source = line.source_id ? sourceById.get(line.source_id) : null;
              return (
                <tr key={line.account_code}>
                  <td className="mono">{line.account_code}</td>
                  <td>{line.account_name}</td>
                  <td>{roleLabel(line.account_type)}</td>
                  <td>
                    {source?.file_name ?? line.source_label}
                    {source ? <small className="source-hash">{formatHash(source.file_hash)}</small> : null}
                  </td>
                  <td className="numeric">{formatCurrency(line.amount, true)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <p className="table-hint">Swipe sideways to inspect all source amount columns.</p>
    </Panel>
  );
}

export function AuditPanel({ workspace }: { workspace: RepoWorkspace }) {
  return (
    <Panel meta={`${workspace.audit_events.length} preserved events`} title="Audit trail">
      <div className="audit-list">
        {workspace.audit_events.length === 0 ? <p className="quiet-note">No audit events yet.</p> : null}
        {[...workspace.audit_events].reverse().map((event) => (
          <article className="audit-row" key={event.id}>
            <strong>{event.sequence_number}. {roleLabel(event.event_type)}</strong>
            <p>{event.message}</p>
            <small>
              {event.actor_name} ({event.actor_email}) - {formatDate(event.occurred_at)} - hash {formatHash(event.event_hash)}
            </small>
          </article>
        ))}
      </div>
    </Panel>
  );
}

function SummaryItem({
  label,
  mono,
  tone = "neutral",
  value,
}: {
  label: string;
  mono?: boolean;
  tone?: "bad" | "good" | "neutral";
  value: string;
}) {
  return (
    <div className={`summary-item summary-item--${tone}`}>
      <span>{label}</span>
      <strong className={mono ? "mono" : undefined}>{value}</strong>
    </div>
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

function ApprovalStep({ complete, label }: { complete: boolean; label: string }) {
  return (
    <div className={complete ? "approval-step approval-step--complete" : "approval-step"}>
      <strong>{label}</strong>
      <span>{complete ? "Done" : "Waiting"}</span>
    </div>
  );
}

function CommitRow({ commit }: { commit: Commit }) {
  return (
    <article className="commit-row">
      <span className="commit-node">{commit.sequence_number}</span>
      <div>
        <strong>{commit.message}</strong>
        <p>
          <span className="mono">{formatHash(commit.snapshot_hash)}</span> - {commit.created_by} - {formatDate(commit.created_at)}
        </p>
      </div>
    </article>
  );
}

function FsLineCard({
  commit,
  importSources,
  line,
}: {
  commit: Commit;
  importSources: ImportSource[];
  line: FinancialStatementLine;
}) {
  const amount = decimal(line.amount);
  const accountCount = line.account_codes.length;
  const sourceById = new Map(importSources.map((source) => [source.id, source]));
  const accountRows = line.account_codes
    .map((accountCode) => {
      const trialBalanceLine = commit.snapshot.trial_balance.find((candidate) => candidate.account_code === accountCode);
      if (!trialBalanceLine) return null;
      const mapping = commit.snapshot.mappings.find((candidate) => candidate.account_code === accountCode);
      const adjustmentAmount = adjustmentTotalForAccount(commit, accountCode);
      const adjustedAmount = decimal(trialBalanceLine.amount) + adjustmentAmount;
      const source = trialBalanceLine.source_id ? sourceById.get(trialBalanceLine.source_id) : null;

      return {
        accountCode,
        adjustedAmount,
        adjustmentAmount,
        assertion: mapping?.assertion ?? "Unmapped",
        source,
        trialBalanceLine,
      };
    })
    .filter((row): row is NonNullable<typeof row> => row !== null);

  return (
    <article className="fs-card">
      <span>{accountCount} accounts</span>
      <strong>{line.fs_line}</strong>
      <p className={amount < 0 ? "negative" : "positive"}>{formatSignedCurrency(line.amount)}</p>
      <details className="fs-trace">
        <summary>Trace source accounts</summary>
        <div className="table-wrap table-wrap--trace">
          <table>
            <thead>
              <tr>
                <th scope="col">Code</th>
                <th scope="col">Account</th>
                <th scope="col">Assertion</th>
                <th scope="col">Source</th>
                <th className="numeric" scope="col">TB</th>
                <th className="numeric" scope="col">Adj</th>
                <th className="numeric" scope="col">Adjusted</th>
              </tr>
            </thead>
            <tbody>
              {accountRows.map((row) => (
                <tr key={row.accountCode}>
                  <td className="mono">{row.accountCode}</td>
                  <td>{row.trialBalanceLine.account_name}</td>
                  <td>{row.assertion}</td>
                  <td>
                    {row.source?.file_name ?? row.trialBalanceLine.source_label}
                    {row.source ? <small className="source-hash">{formatHash(row.source.file_hash)}</small> : null}
                  </td>
                  <td className="numeric">{formatCurrency(row.trialBalanceLine.amount, true)}</td>
                  <td className="numeric">{formatSignedCurrency(row.adjustmentAmount.toFixed(2))}</td>
                  <td className="numeric">{formatSignedCurrency(row.adjustedAmount.toFixed(2))}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
    </article>
  );
}

function adjustmentTotalForAccount(commit: Commit, accountCode: string) {
  return commit.snapshot.adjustments
    .flatMap((adjustment) => adjustment.lines)
    .filter((line) => line.account_code === accountCode)
    .reduce((total, line) => total + decimal(line.amount), 0);
}
