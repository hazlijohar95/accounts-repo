import type {
  Approval,
  ApprovalPayload,
  Commit,
  CorrectionCommitPayload,
  ImportWorkspacePayload,
  LegalEntityRepo,
  RepoWorkspace,
  ResolveReviewQueryPayload,
  ReviewPack,
  ReviewQuery,
  ReviewQueryPayload,
} from "./types";

const API_BASE = import.meta.env.VITE_API_BASE_URL ?? "";

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers);
  headers.set("Content-Type", "application/json");
  if (import.meta.env.VITE_DEV_AUTH_EMAIL) {
    headers.set("x-dev-user-id", import.meta.env.VITE_DEV_AUTH_ID ?? import.meta.env.VITE_DEV_AUTH_EMAIL);
    headers.set("x-dev-user-name", import.meta.env.VITE_DEV_AUTH_NAME ?? import.meta.env.VITE_DEV_AUTH_EMAIL);
    headers.set("x-dev-user-email", import.meta.env.VITE_DEV_AUTH_EMAIL);
  }

  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    credentials: "include",
    headers,
  });

  if (!response.ok) {
    const errorPayload = (await response.json().catch(() => ({ error: response.statusText }))) as {
      error?: string;
    };
    throw new Error(errorPayload.error ?? `Request failed with ${response.status}`);
  }

  return (await response.json()) as T;
}

export function listRepos(): Promise<LegalEntityRepo[]> {
  return requestJson<LegalEntityRepo[]>("/api/repos");
}

export function importWorkspace(payload: ImportWorkspacePayload): Promise<RepoWorkspace> {
  return requestJson<RepoWorkspace>("/api/imports/year-end-review-pack", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function getRepoWorkspace(repoId: string): Promise<RepoWorkspace> {
  return requestJson<RepoWorkspace>(`/api/repos/${repoId}`);
}

export function getReviewPack(reviewPackId: string): Promise<ReviewPack> {
  return requestJson<ReviewPack>(`/api/review-packs/${reviewPackId}`);
}

export function approveReviewer(reviewPackId: string, payload: ApprovalPayload): Promise<Approval> {
  return requestJson<Approval>(`/api/review-packs/${reviewPackId}/reviewer-approval`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function signClient(reviewPackId: string, payload: ApprovalPayload): Promise<Approval> {
  return requestJson<Approval>(`/api/review-packs/${reviewPackId}/client-signoff`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function openReviewQuery(reviewPackId: string, payload: ReviewQueryPayload): Promise<ReviewQuery> {
  return requestJson<ReviewQuery>(`/api/review-packs/${reviewPackId}/queries`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function resolveReviewQuery(
  reviewPackId: string,
  queryId: string,
  payload: ResolveReviewQueryPayload,
): Promise<ReviewQuery> {
  return requestJson<ReviewQuery>(`/api/review-packs/${reviewPackId}/queries/${queryId}/resolve`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export function exportSignedPack(reviewPackId: string): Promise<unknown> {
  return requestJson<unknown>(`/api/review-packs/${reviewPackId}/signed-export`, {
    method: "POST",
  });
}

export function commitCorrection(
  repoId: string,
  branchId: string,
  payload: CorrectionCommitPayload,
): Promise<Commit> {
  return requestJson<Commit>(`/api/repos/${repoId}/branches/${branchId}/correction-commits`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}
