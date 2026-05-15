import type { BranchStatus, ReviewStatus } from "./types";

const currencyFormatter = new Intl.NumberFormat("en-MY", {
  style: "currency",
  currency: "MYR",
  maximumFractionDigits: 0,
});

const exactCurrencyFormatter = new Intl.NumberFormat("en-MY", {
  style: "currency",
  currency: "MYR",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

export function decimal(value: string): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function formatCurrency(value: string, exact = false): string {
  const formatter = exact ? exactCurrencyFormatter : currencyFormatter;
  return formatter.format(decimal(value));
}

export function formatSignedCurrency(value: string): string {
  const amount = decimal(value);
  const prefix = amount > 0 ? "+" : "";
  return `${prefix}${formatCurrency(value, true)}`;
}

export function formatHash(hash: string): string {
  return hash.length > 12 ? `${hash.slice(0, 12)}` : hash;
}

export function formatDate(value: string): string {
  return new Intl.DateTimeFormat("en-MY", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  }).format(new Date(value));
}

export function reviewStatusLabel(status: ReviewStatus): string {
  switch (status) {
    case "in_review":
      return "In review";
    case "reviewer_approved":
      return "Reviewer approved";
    case "signed":
      return "Signed and frozen";
  }
}

export function branchStatusLabel(status: BranchStatus): string {
  switch (status) {
    case "working":
      return "Working branch";
    case "in_review":
      return "In review";
    case "frozen":
      return "Frozen history";
  }
}

export function roleLabel(role: string): string {
  return role
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function absoluteDecimal(value: string): string {
  return Math.abs(decimal(value)).toString();
}
