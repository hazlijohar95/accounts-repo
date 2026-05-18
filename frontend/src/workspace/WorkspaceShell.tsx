import { useEffect, useRef, useState } from "react";

import { branchStatusLabel, formatDate, formatHash, reviewStatusLabel, roleLabel } from "../format";
import { DialogFrame, KeyValue, PreferenceRow, SegmentedControl, SheetFrame, StatusPill, SummaryTile, ThemeToggle } from "../ui/primitives";
import type { ColorMode, RadiusPreset, ThemeControls } from "../theme/useColorTheme";
import type { Commit, LegalEntityRepo, RepoWorkspace } from "../types";
import { workspaceSourceLabel } from "./helpers";
import { WORKSPACE_TABS, type WorkspaceTab } from "./tabs";

export function RepoSidebar({
  actionPending,
  currentUser,
  headCommit,
  onRepoSelect,
  onNewImport,
  onRepoQueryChange,
  onSignOut,
  repoQuery,
  repos,
  selectedRepoId,
  workspace,
}: {
  actionPending: string | null;
  currentUser: { name: string; email: string };
  headCommit: Commit;
  onNewImport: () => void;
  onRepoQueryChange: (query: string) => void;
  onRepoSelect: (repoId: string) => Promise<void>;
  onSignOut: () => void;
  repoQuery: string;
  repos: LegalEntityRepo[];
  selectedRepoId: string | null;
  workspace: RepoWorkspace;
}) {
  const sourceLabel = workspaceSourceLabel(headCommit.snapshot.trial_balance);
  const normalizedQuery = repoQuery.trim().toLowerCase();
  const visibleRepos = normalizedQuery
    ? repos.filter((repo) => `${repo.name} ${repo.registration_number} ${repo.summary.active_branch_label}`.toLowerCase().includes(normalizedQuery))
    : repos;

  return (
    <aside className="repo-sidebar" aria-label="Repository sidebar">
      <div className="brand-block">
        <span className="brand-mark">AR</span>
        <div>
          <strong>Accounts Repo</strong>
          <small>Financial source control</small>
        </div>
      </div>

      <section className="sidebar-section">
        <div className="sidebar-section__head">
          <p className="section-label">Repos</p>
          <span className="repo-count">{visibleRepos.length}/{repos.length}</span>
        </div>
        <label className="sidebar-search">
          <span>Find repo</span>
          <input
            aria-label="Filter repositories"
            onChange={(event) => onRepoQueryChange(event.target.value)}
            placeholder="Entity, branch, reg no"
            value={repoQuery}
          />
        </label>
        <nav className="repo-list">
          {visibleRepos.map((repo) => {
            const isActive = repo.id === selectedRepoId;

            return (
              <button
                aria-current={isActive ? "page" : undefined}
                className={isActive ? "repo-button repo-button--active" : "repo-button"}
                disabled={actionPending !== null}
                key={repo.id}
                onClick={() => void onRepoSelect(repo.id)}
                type="button"
              >
                <span>{repo.name}</span>
                <small>{repo.summary.active_branch_label}</small>
              </button>
            );
          })}
          {visibleRepos.length === 0 ? <p className="quiet-note">No repos match that filter.</p> : null}
        </nav>
        <button className="secondary-button" disabled={actionPending !== null} onClick={onNewImport} type="button">
          Import another repo
        </button>
      </section>

      <section className="source-card" aria-label="Source data notice">
        <strong>Imported source data</strong>
        <p>{sourceLabel}</p>
      </section>

      <section className="sidebar-section">
        <p className="section-label">Info</p>
        <KeyValue label="Entity" value={workspace.repo.entity_type} />
        <KeyValue label="Branch" value={workspace.branch.label} />
        <KeyValue label="Head" mono value={formatHash(headCommit.snapshot_hash)} />
        <KeyValue label="Status" value={branchStatusLabel(workspace.branch.status)} />
      </section>

      <section className="sidebar-section sidebar-section--bottom">
        <p className="section-label">Signed in</p>
        <KeyValue label={currentUser.name} value={currentUser.email} />
        <button className="secondary-button" onClick={onSignOut} type="button">
          Sign out
        </button>
      </section>

      <section className="sidebar-section">
        <p className="section-label">Custody</p>
        {workspace.repo.collaborators.map((collaborator) => (
          <KeyValue
            key={collaborator.user_id}
            label={collaborator.display_name}
            value={roleLabel(collaborator.role)}
          />
        ))}
      </section>
    </aside>
  );
}

export function RepoHeader({ headCommit, workspace }: { headCommit: Commit; workspace: RepoWorkspace }) {
  return (
    <header className="repo-header">
      <div className="repo-heading">
        <p className="repo-breadcrumb">Amjad & Hazli / {workspace.repo.entity_type}</p>
        <h1>{workspace.repo.name}</h1>
        <p>{workspace.repo.registration_number}</p>
      </div>
      <div className="repo-badges" aria-label="Repository status">
        <StatusPill status={workspace.review_pack.status} />
        <span className="badge">{workspace.branch.label}</span>
        <span className="badge mono">{formatHash(headCommit.snapshot_hash)}</span>
      </div>
    </header>
  );
}

export function RepoTabs({
  activeTab,
  auditCount,
  commitCount,
  fsLineCount,
  onTabChange,
  tbCount,
}: {
  activeTab: WorkspaceTab;
  auditCount: number;
  commitCount: number;
  fsLineCount: number;
  onTabChange: (tab: WorkspaceTab) => void;
  tbCount: number;
}) {
  const counts: Partial<Record<WorkspaceTab, number>> = {
    audit: auditCount,
    commits: commitCount,
    statements: fsLineCount,
    "trial-balance": tbCount,
  };

  return (
    <nav className="repo-tabs" aria-label="Review workspace tabs" role="tablist">
      {WORKSPACE_TABS.map((item) => {
        const active = item.tab === activeTab;
        const count = counts[item.tab];
        return (
          <button
            aria-controls={`panel-${item.tab}`}
            aria-selected={active}
            className={active ? "tab-button tab-button--active" : "tab-button"}
            id={`tab-${item.tab}`}
            key={item.tab}
            onClick={() => onTabChange(item.tab)}
            role="tab"
            type="button"
          >
            {item.label}
            {count !== undefined ? <span>{count}</span> : null}
          </button>
        );
      })}
    </nav>
  );
}

export function AppNavbar({
  activityCount,
  currentUser,
  mode,
  onOpenActivity,
  onOpenCommand,
  onOpenSettings,
  onSignOut,
  onToggleTheme,
}: {
  activityCount: number;
  currentUser: { name: string; email: string };
  mode: ColorMode;
  onOpenActivity: () => void;
  onOpenCommand: () => void;
  onOpenSettings: () => void;
  onSignOut: () => void;
  onToggleTheme: () => void;
}) {
  return (
    <header className="hub-navbar">
      <nav aria-label="Global navigation">
        <div className="hub-brand" aria-label="Accounts Repo">
          <span className="hub-glyph" aria-hidden="true">
            <span />
            <span />
            <span />
          </span>
          <span>ACCOUNTS-REPO.</span>
        </div>

        <button className="command-trigger" onClick={onOpenCommand} type="button">
          <span>Search repos, tabs, actions</span>
          <kbd>Cmd K</kbd>
        </button>

        <div className="hub-actions">
          <button className="nav-icon-button nav-icon-button--badge" onClick={onOpenActivity} type="button">
            Activity
            {activityCount > 0 ? <span>{activityCount}</span> : null}
          </button>
          <button className="nav-icon-button" onClick={onOpenSettings} type="button">Settings</button>
          <ThemeToggle mode={mode} onToggle={onToggleTheme} />
          <span className="user-chip" title={currentUser.email}>{currentUser.name}</span>
          <button className="nav-icon-button" onClick={onSignOut} type="button">Sign out</button>
        </div>
      </nav>
    </header>
  );
}

export function CommandPalette({
  activityCount,
  actionPending,
  activeTab,
  mode,
  onNewImport,
  onOpenActivity,
  onOpenChange,
  onOpenSettings,
  onRepoSelect,
  onSignOut,
  onTabChange,
  onToggleTheme,
  open,
  repos,
  selectedRepoId,
  workspace,
}: {
  activityCount: number;
  actionPending: string | null;
  activeTab: WorkspaceTab;
  mode: ColorMode;
  onNewImport: () => void;
  onOpenActivity: () => void;
  onOpenChange: (open: boolean) => void;
  onOpenSettings: () => void;
  onRepoSelect: (repoId: string) => Promise<void>;
  onSignOut: () => void;
  onTabChange: (tab: WorkspaceTab) => void;
  onToggleTheme: () => void;
  open: boolean;
  repos: LegalEntityRepo[];
  selectedRepoId: string | null;
  workspace: RepoWorkspace | null;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const tabActions = WORKSPACE_TABS.map((item, index) => ({
    id: `tab:${item.tab}`,
    group: "Workspace",
    label: `Open ${item.label}`,
    detail: `Shortcut ${index + 1}${item.tab === activeTab ? " active" : ""}`,
    run: () => onTabChange(item.tab),
  }));
  const repoActions = repos.map((repo) => ({
    id: `repo:${repo.id}`,
    group: "Repos",
    label: repo.name,
    detail: `${repo.summary.active_branch_label}${repo.id === selectedRepoId ? " active" : ""}`,
    run: () => void onRepoSelect(repo.id),
  }));
  const actions = [
    {
      id: "action:import",
      group: "Actions",
      label: "Import another repo",
      detail: "Start a new source-controlled review pack",
      run: onNewImport,
    },
    {
      id: "action:activity",
      group: "Actions",
      label: "Open activity sheet",
      detail: activityCount > 0 ? `${activityCount} open review signals` : "Audit events and open queries",
      run: onOpenActivity,
    },
    {
      id: "action:settings",
      group: "Preferences",
      label: "Open settings",
      detail: "Theme, radius, account, and workspace preferences",
      run: onOpenSettings,
    },
    {
      id: "action:theme",
      group: "Preferences",
      label: `Switch to ${mode === "dark" ? "light" : "dark"} mode`,
      detail: "Better Hub theme token set",
      run: onToggleTheme,
    },
    {
      id: "action:signout",
      group: "Account",
      label: "Sign out",
      detail: "End this Better Auth session",
      run: onSignOut,
    },
    ...tabActions,
    ...repoActions,
  ];
  const normalizedQuery = query.trim().toLowerCase();
  const filteredActions = normalizedQuery
    ? actions.filter((item) => `${item.group} ${item.label} ${item.detail}`.toLowerCase().includes(normalizedQuery))
    : actions;
  const selectedAction = filteredActions[selectedIndex];

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setSelectedIndex(0);
    window.requestAnimationFrame(() => inputRef.current?.focus());
  }, [open]);

  useEffect(() => {
    if (!open) return;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = "";
    };
  }, [open]);

  useEffect(() => {
    if (selectedIndex >= filteredActions.length) {
      setSelectedIndex(Math.max(0, filteredActions.length - 1));
    }
  }, [filteredActions.length, selectedIndex]);

  if (!open) return null;

  return (
    <div className="command-overlay" onMouseDown={() => onOpenChange(false)} role="presentation">
      <section
        aria-label="Command menu"
        aria-modal="true"
        className="command-panel"
        onKeyDown={(event) => {
          if (event.key === "ArrowDown") {
            event.preventDefault();
            setSelectedIndex((index) => Math.min(index + 1, Math.max(0, filteredActions.length - 1)));
          } else if (event.key === "ArrowUp") {
            event.preventDefault();
            setSelectedIndex((index) => Math.max(index - 1, 0));
          } else if (event.key === "Enter" && selectedAction) {
            event.preventDefault();
            onOpenChange(false);
            selectedAction.run();
          } else if (event.key === "Escape") {
            event.preventDefault();
            onOpenChange(false);
          }
        }}
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
      >
        <div className="command-search-row">
          <span aria-hidden="true">/</span>
          <input
            aria-label="Search commands"
            disabled={actionPending !== null}
            onChange={(event) => {
              setQuery(event.target.value);
              setSelectedIndex(0);
            }}
            placeholder={workspace ? `Search ${workspace.repo.name}` : "Search actions"}
            ref={inputRef}
            value={query}
          />
        </div>
        <div className="command-results" role="listbox">
          {filteredActions.map((item, index) => (
            <button
              aria-selected={index === selectedIndex}
              className={index === selectedIndex ? "command-item command-item--active" : "command-item"}
              key={item.id}
              onClick={() => {
                onOpenChange(false);
                item.run();
              }}
              onMouseEnter={() => setSelectedIndex(index)}
              role="option"
              type="button"
            >
              <span>
                <strong>{item.label}</strong>
                <small>{item.detail}</small>
              </span>
              <em>{item.group}</em>
            </button>
          ))}
          {filteredActions.length === 0 ? <p className="command-empty">No commands found.</p> : null}
        </div>
      </section>
    </div>
  );
}

export function SettingsDialog({
  currentUser,
  onOpenChange,
  open,
  theme,
  workspace,
}: {
  currentUser: { name: string; email: string };
  onOpenChange: (open: boolean) => void;
  open: boolean;
  theme: ThemeControls;
  workspace: RepoWorkspace;
}) {
  const [activeTab, setActiveTab] = useState<"general" | "appearance" | "account">("general");
  const currentTheme = theme.themes.find((candidate) => candidate.id === theme.themeId) ?? theme.themes[0];

  return (
    <DialogFrame description="Manage product preferences, theme, radius, and signed-in identity." onOpenChange={onOpenChange} open={open} title="Settings">
      <div className="settings-shell">
        <header className="settings-header">
          <div>
            <h2>Settings</h2>
            <p>Manage preferences, workspace behavior, and account context.</p>
          </div>
          <button className="nav-icon-button" onClick={() => onOpenChange(false)} type="button">Close</button>
        </header>

        <nav className="settings-tabs" aria-label="Settings sections">
          {[
            ["general", "General"],
            ["appearance", "Appearance"],
            ["account", "Account"],
          ].map(([id, label]) => (
            <button
              aria-current={activeTab === id ? "page" : undefined}
              className={activeTab === id ? "settings-tab settings-tab--active" : "settings-tab"}
              key={id}
              onClick={() => setActiveTab(id as typeof activeTab)}
              type="button"
            >
              {label}
            </button>
          ))}
        </nav>

        <section className="settings-body">
          {activeTab === "general" ? (
            <div className="settings-section">
              <PreferenceRow label="Active repo" value={workspace.repo.name} />
              <PreferenceRow label="Branch" value={workspace.branch.label} />
              <PreferenceRow label="Review status" value={reviewStatusLabel(workspace.review_pack.status)} />
              <PreferenceRow label="Open queries" value={String(workspace.review_pack.open_queries.filter((query) => query.status === "open").length)} />
              <div className="settings-callout">
                <strong>Better Hub pattern target</strong>
                <p>Dense workspace, command-first navigation, sheet/modal actions, and themeable tokens. This app now uses those primitives without changing financial-review behavior.</p>
              </div>
            </div>
          ) : null}

          {activeTab === "appearance" ? (
            <div className="settings-section">
              <div className="settings-group">
                <div>
                  <h3>Color theme</h3>
                  <p>Built from Better Hub's real theme token definitions.</p>
                </div>
                <div className="theme-grid">
                  {theme.themes.map((item) => (
                    <button
                      aria-pressed={theme.themeId === item.id}
                      className={theme.themeId === item.id ? "theme-card theme-card--active" : "theme-card"}
                      key={item.id}
                      onClick={() => theme.setThemeId(item.id)}
                      type="button"
                    >
                      <span className="theme-swatch-row" aria-hidden="true">
                        <i style={{ background: item[theme.mode]["--background"] }} />
                        <i style={{ background: item[theme.mode]["--secondary"] }} />
                        <i style={{ background: item[theme.mode]["--primary"] }} />
                      </span>
                      <strong>{item.name}</strong>
                      <small>{item.description}</small>
                    </button>
                  ))}
                </div>
              </div>

              <div className="settings-group settings-group--split">
                <div>
                  <h3>Mode</h3>
                  <p>{currentTheme.name} in {theme.mode} mode.</p>
                </div>
                <SegmentedControl
                  label="Color mode"
                  options={["dark", "light"]}
                  value={theme.mode}
                  onChange={(value) => theme.setMode(value as ColorMode)}
                />
              </div>

              <div className="settings-group settings-group--split">
                <div>
                  <h3>Radius</h3>
                  <p>Matches Better Hub's border-radius presets.</p>
                </div>
                <SegmentedControl
                  label="Border radius"
                  options={["default", "small", "medium", "large"]}
                  value={theme.radius}
                  onChange={(value) => theme.setRadius(value as RadiusPreset)}
                />
              </div>
            </div>
          ) : null}

          {activeTab === "account" ? (
            <div className="settings-section">
              <PreferenceRow label="Signed in as" value={currentUser.name} />
              <PreferenceRow label="Email" mono value={currentUser.email} />
              <PreferenceRow label="Custody roles" value={workspace.repo.collaborators.map((collaborator) => roleLabel(collaborator.role)).join(", ")} />
              <div className="settings-callout">
                <strong>Role-bound evidence</strong>
                <p>Approvals, corrections, query resolution, and signed exports stay tied to the authenticated identity, not the local theme/preferences layer.</p>
              </div>
            </div>
          ) : null}
        </section>
      </div>
    </DialogFrame>
  );
}

export function ActivitySheet({
  onOpenChange,
  open,
  workspace,
}: {
  onOpenChange: (open: boolean) => void;
  open: boolean;
  workspace: RepoWorkspace;
}) {
  const openQueries = workspace.review_pack.open_queries.filter((query) => query.status === "open");
  const recentAuditEvents = [...workspace.audit_events].reverse().slice(0, 12);

  return (
    <SheetFrame description="Review queries and recent audit events." onOpenChange={onOpenChange} open={open} title="Activity">
      <header className="sheet-header">
        <div>
          <h2>Activity</h2>
          <p>{workspace.repo.name}</p>
        </div>
        <button className="nav-icon-button" onClick={() => onOpenChange(false)} type="button">Close</button>
      </header>

      <div className="sheet-body">
        <section className="sheet-summary-grid" aria-label="Activity summary">
          <SummaryTile label="Open queries" value={String(openQueries.length)} />
          <SummaryTile label="Audit events" value={String(workspace.audit_events.length)} />
          <SummaryTile label="Commits" value={String(workspace.commits.length)} />
        </section>

        <section className="activity-section">
          <div className="activity-section__head">
            <h3>Review queries</h3>
            <span>{workspace.review_pack.open_queries.length} total</span>
          </div>
          {workspace.review_pack.open_queries.length > 0 ? (
            workspace.review_pack.open_queries.map((query) => (
              <article className={query.status === "open" ? "activity-row activity-row--unread" : "activity-row"} key={query.id}>
                <div>
                  <strong>{query.title}</strong>
                  <p>{query.status === "open" ? "Open" : "Resolved"} - assigned to {query.assigned_to}</p>
                </div>
                {query.status === "open" ? <span className="activity-dot" aria-label="Open query" /> : null}
              </article>
            ))
          ) : (
            <p className="command-empty">All caught up.</p>
          )}
        </section>

        <section className="activity-section">
          <div className="activity-section__head">
            <h3>Audit trail</h3>
            <span>{recentAuditEvents.length} shown</span>
          </div>
          {recentAuditEvents.length > 0 ? (
            recentAuditEvents.map((event) => (
              <article className="activity-row" key={event.id}>
                <div>
                  <strong>{event.sequence_number}. {roleLabel(event.event_type)}</strong>
                  <p>{event.message}</p>
                  <small>{event.actor_name} - {formatDate(event.occurred_at)} - {formatHash(event.event_hash)}</small>
                </div>
              </article>
            ))
          ) : (
            <p className="command-empty">No audit events yet.</p>
          )}
        </section>
      </div>
    </SheetFrame>
  );
}
