import type {
  Approval,
  ApprovalPayload,
  Commit,
  CorrectionCommitPayload,
  LegalEntityRepo,
  RepoWorkspace,
  ReviewPack,
} from "./types";

const API_BASE = import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:8080";

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      "Content-Type": "application/json",
      ...init?.headers,
    },
    ...init,
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
