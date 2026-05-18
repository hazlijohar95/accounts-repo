import { useEffect, useState } from "react";
import type { FormEvent } from "react";

import {
  approveReviewer,
  commitCorrection,
  exportSignedPack,
  getRepoWorkspace,
  importWorkspace,
  listRepos,
  openReviewQuery,
  resolveReviewQuery,
  signClient,
} from "./api";
import { authClient, useAuthSession } from "./auth-client";
import { roleLabel } from "./format";
import { ImportEmptyState } from "./import/ImportEmptyState";
import { useColorTheme, type ThemeControls } from "./theme/useColorTheme";
import { LoadingScreen, ThemeToggle, ToastStack } from "./ui/primitives";
import type {
  Commit,
  CorrectionCommitPayload,
  ImportWorkspacePayload,
  LegalEntityRepo,
  RepoWorkspace,
  ReviewQuery,
} from "./types";
import { AuditPanel, CommitPanel, ReviewWorkspace, StatementsPanel, TrialBalancePanel } from "./workspace/ReviewWorkspace";
import { ActivitySheet, AppNavbar, CommandPalette, RepoHeader, RepoSidebar, RepoTabs, SettingsDialog } from "./workspace/WorkspaceShell";
import { WORKSPACE_TABS, type WorkspaceTab } from "./workspace/tabs";

export function App() {
  const session = useAuthSession();
  const theme = useColorTheme();

  if (session.isPending) return <LoadingScreen />;

  if (!session.data?.user) {
    return <AuthScreen onAuthChanged={() => void session.refetch()} theme={theme} />;
  }

  return <WorkspaceApp currentUser={session.data.user} theme={theme} />;
}

function WorkspaceApp({
  currentUser,
  theme,
}: {
  currentUser: { id: string; name: string; email: string };
  theme: ThemeControls;
}) {
  const [repos, setRepos] = useState<LegalEntityRepo[]>([]);
  const [selectedRepoId, setSelectedRepoId] = useState<string | null>(null);
  const [workspace, setWorkspace] = useState<RepoWorkspace | null>(null);
  const [activeTab, setActiveTab] = useState<WorkspaceTab>("review");
  const [commandOpen, setCommandOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [activityOpen, setActivityOpen] = useState(false);
  const [repoQuery, setRepoQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [actionPending, setActionPending] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function loadInitial(isActive: () => boolean = () => true) {
    try {
      setLoading(true);
      setError(null);
      const repoList = await listRepos();
      if (!isActive()) return;

      setRepos(repoList);
      const firstRepoId = repoList[0]?.id ?? null;
      setSelectedRepoId(firstRepoId);
      setActiveTab("review");

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
      setNotice(null);
      await action();
      await reloadWorkspace();
      setNotice(actionSuccessMessage(label));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Action failed");
      setNotice(null);
    } finally {
      setActionPending(null);
    }
  }

  async function handleImport(payload: ImportWorkspacePayload) {
    try {
      setActionPending("import");
      setError(null);
      const importedWorkspace = await importWorkspace(payload);
      const nextRepos = await listRepos();
      setRepos(nextRepos);
      setSelectedRepoId(importedWorkspace.repo.id);
      setWorkspace(importedWorkspace);
      setActiveTab("review");
      setNotice("Imported review pack and created the first source-controlled commit.");
      scrollToTop();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Import failed");
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
      setActiveTab("review");
      scrollToTop();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "Failed to load selected repo");
    } finally {
      setActionPending(null);
    }
  }

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      const key = event.key.toLowerCase();

      if ((event.metaKey || event.ctrlKey) && key === "k") {
        event.preventDefault();
        setCommandOpen((open) => !open);
        return;
      }

      if (event.key === "Escape" && commandOpen) {
        event.preventDefault();
        setCommandOpen(false);
        return;
      }

      if (isTypingTarget(event.target)) return;

      const tabNumber = Number(event.key);
      if (tabNumber >= 1 && tabNumber <= WORKSPACE_TABS.length) {
        event.preventDefault();
        setActiveTab(WORKSPACE_TABS[tabNumber - 1].tab);
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [commandOpen]);

  if (loading) return <LoadingScreen />;

  if (!workspace) {
    return (
      <ImportEmptyState
        error={error}
        importing={actionPending === "import"}
        onImport={(payload) => void handleImport(payload)}
        onRetry={() => void loadInitial()}
        currentUser={currentUser}
      />
    );
  }

  const headCommit = workspace.commits.find((commit) => commit.id === workspace.branch.head_commit_id) ??
    workspace.commits[workspace.commits.length - 1];
  const firstCommit = workspace.commits.find((commit) => commit.id === workspace.fs_impact_diff.from_commit_id) ??
    workspace.commits[0];

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
  const openQueryCount = workspace.review_pack.open_queries.filter((query) => query.status === "open").length;

  return (
    <>
      <AppNavbar
        activityCount={openQueryCount}
        currentUser={currentUser}
        mode={theme.mode}
        onOpenActivity={() => setActivityOpen(true)}
        onOpenCommand={() => setCommandOpen(true)}
        onOpenSettings={() => setSettingsOpen(true)}
        onSignOut={() => void authClient.signOut().then(() => window.location.reload())}
        onToggleTheme={theme.toggleMode}
      />
      <SettingsDialog
        currentUser={currentUser}
        onOpenChange={setSettingsOpen}
        open={settingsOpen}
        theme={theme}
        workspace={workspace}
      />
      <ActivitySheet onOpenChange={setActivityOpen} open={activityOpen} workspace={workspace} />
      <CommandPalette
        activityCount={openQueryCount}
        actionPending={actionPending}
        activeTab={activeTab}
        mode={theme.mode}
        onOpenActivity={() => setActivityOpen(true)}
        onOpenSettings={() => setSettingsOpen(true)}
        onNewImport={() => {
          setWorkspace(null);
          setSelectedRepoId(null);
          setActiveTab("review");
          setCommandOpen(false);
          scrollToTop();
        }}
        onOpenChange={setCommandOpen}
        onRepoSelect={handleRepoSelect}
        onSignOut={() => void authClient.signOut().then(() => window.location.reload())}
        onTabChange={setActiveTab}
        onToggleTheme={theme.toggleMode}
        open={commandOpen}
        repos={repos}
        selectedRepoId={selectedRepoId}
        workspace={workspace}
      />
      <main className="app-shell" aria-busy={actionPending !== null}>
        <RepoSidebar
          actionPending={actionPending}
          currentUser={currentUser}
          headCommit={headCommit}
          onNewImport={() => {
            setWorkspace(null);
            setSelectedRepoId(null);
            setActiveTab("review");
            scrollToTop();
          }}
          onRepoQueryChange={setRepoQuery}
          onRepoSelect={handleRepoSelect}
          onSignOut={() => void authClient.signOut().then(() => window.location.reload())}
          repoQuery={repoQuery}
          repos={repos}
          selectedRepoId={selectedRepoId}
          workspace={workspace}
        />

        <section className="repo-page">
          <RepoHeader headCommit={headCommit} workspace={workspace} />
          <RepoTabs
            activeTab={activeTab}
            auditCount={workspace.audit_events.length}
            commitCount={workspace.commits.length}
            fsLineCount={headCommit.snapshot.fs_lines.length}
            onTabChange={setActiveTab}
            tbCount={headCommit.snapshot.trial_balance.length}
          />

          <ToastStack error={error} notice={notice} onDismiss={() => {
            setError(null);
            setNotice(null);
          }} />

          <section aria-labelledby={`tab-${activeTab}`} className="tab-panel" id={`panel-${activeTab}`} role="tabpanel">
            {activeTab === "review" ? (
              <ReviewWorkspace
                actionPending={actionPending}
                branchFrozen={branchFrozen}
                firstCommit={firstCommit}
                headCommit={headCommit}
                onApprove={(note) =>
                  void runAction("approve", async () => {
                    await approveReviewer(workspace.review_pack.id, {
                      note,
                    });
                  })
                }
                onCommitCorrection={(payload) =>
                  void runAction("correction", async () => {
                    await commitCorrection(workspace.repo.id, workspace.branch.id, payload);
                  })
                }
                onOpenQuery={(title) =>
                  void runAction("open-query", async () => {
                    await openReviewQuery(workspace.review_pack.id, {
                      title,
                      assigned_to: collaboratorName(workspace, "preparer"),
                    });
                  })
                }
                onResolveQuery={(query) =>
                  void runAction(`resolve:${query.id}`, async () => {
                    await resolveReviewQuery(workspace.review_pack.id, query.id, {
                      note: `Resolved by ${query.assigned_to}`,
                    });
                  })
                }
                onExportSignedPack={() =>
                  void runAction("export", async () => {
                    const payload = await exportSignedPack(workspace.review_pack.id);
                    downloadJson(`${workspace.repo.name}-${workspace.branch.label}-signed-pack.json`, payload);
                  })
                }
                onSign={(note) =>
                  void runAction("sign", async () => {
                    await signClient(workspace.review_pack.id, {
                      note,
                    });
                  })
                }
                currentUser={currentUser}
                workspace={workspace}
              />
            ) : null}

            {activeTab === "commits" ? <CommitPanel commits={workspace.commits} /> : null}
            {activeTab === "statements" ? (
              <StatementsPanel commit={headCommit} importSources={workspace.import_sources} lines={headCommit.snapshot.fs_lines} />
            ) : null}
            {activeTab === "trial-balance" ? <TrialBalancePanel commit={headCommit} importSources={workspace.import_sources} /> : null}
            {activeTab === "audit" ? <AuditPanel workspace={workspace} /> : null}
          </section>
        </section>
      </main>
    </>
  );
}

function AuthScreen({ onAuthChanged, theme }: { onAuthChanged: () => void; theme: ThemeControls }) {
  const showLocalDefaults = import.meta.env.DEV;
  const [mode, setMode] = useState<"sign-in" | "sign-up">("sign-in");
  const [name, setName] = useState(showLocalDefaults ? "Aina Rahman" : "");
  const [email, setEmail] = useState(showLocalDefaults ? "aina@ahadvisory.test" : "");
  const [password, setPassword] = useState(showLocalDefaults ? "accounts-repo-demo-2026" : "");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPending(true);
    setError(null);
    setNotice(null);

    const result = mode === "sign-up"
      ? await authClient.signUp.email({ email, password, name })
      : await authClient.signIn.email({ email, password });

    setPending(false);
    if (result.error) {
      setError(result.error.message ?? "Authentication failed");
      return;
    }

    if (mode === "sign-up") {
      setNotice("Account created. Verify your email, then sign in.");
      setMode("sign-in");
      return;
    }

    onAuthChanged();
  }

  return (
    <main className="empty-state empty-state--auth">
      <div className="auth-toolbar">
        <span>ACCOUNTS-REPO.</span>
        <ThemeToggle mode={theme.mode} onToggle={theme.toggleMode} />
      </div>
      <section className="import-intro">
        <p className="eyebrow">Accounts Repo</p>
        <h1>Sign in to a role-bound financial repo.</h1>
        <p className="empty-copy">
          Approvals, corrections, and signed exports are tied to your authenticated identity.
        </p>
      </section>

      <form className="import-panel" onSubmit={(event) => void handleSubmit(event)}>
        <div>
          <p className="section-label">Better Auth</p>
          <h2>{mode === "sign-up" ? "Create account" : "Sign in"}</h2>
        </div>
        {mode === "sign-up" ? (
          <label>
            Name
            <input required value={name} onChange={(event) => setName(event.target.value)} />
          </label>
        ) : null}
        <label>
          Email
          <input required type="email" value={email} onChange={(event) => setEmail(event.target.value)} />
        </label>
        <label>
          Password
          <input required minLength={12} type="password" value={password} onChange={(event) => setPassword(event.target.value)} />
        </label>
        {error ? <p className="error-copy" role="alert">{error}</p> : null}
        {notice ? <p className="success-copy" role="status">{notice}</p> : null}
        <button className="primary-button" disabled={pending} type="submit">
          {pending ? "Checking..." : mode === "sign-up" ? "Create account" : "Sign in"}
        </button>
        <button
          className="secondary-button"
          onClick={() => setMode(mode === "sign-up" ? "sign-in" : "sign-up")}
          type="button"
        >
          {mode === "sign-up" ? "Use existing account" : "Create a new account"}
        </button>
      </form>
    </main>
  );
}

function actionSuccessMessage(label: string) {
  if (label === "approve") return "Reviewer approval recorded against the current commit.";
  if (label === "sign") return "Client sign-off recorded and branch status refreshed.";
  if (label === "correction") return "Correction commit appended and financial statement impact refreshed.";
  if (label === "open-query") return "Review query opened and assigned.";
  if (label.startsWith("resolve:")) return "Review query resolved.";
  if (label === "export") return "Signed export generated.";
  return "Workspace updated.";
}

function collaboratorName(workspace: RepoWorkspace, role: "preparer" | "reviewer" | "client_signer") {
  return workspace.repo.collaborators.find((collaborator) => collaborator.role === role)?.display_name ?? roleLabel(role);
}

function downloadJson(filename: string, payload: unknown) {
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename.replaceAll(/[^a-z0-9_.-]+/gi, "-").toLowerCase();
  link.click();
  URL.revokeObjectURL(url);
}

function scrollToTop() {
  if (import.meta.env.MODE === "test") return;
  window.requestAnimationFrame(() => window.scrollTo({ top: 0, left: 0, behavior: "auto" }));
}

function isTypingTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) return false;
  return target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable;
}
