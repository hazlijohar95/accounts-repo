import { useEffect } from "react";
import type { ReactNode } from "react";

import { reviewStatusLabel } from "../format";
import type { ReviewStatus } from "../types";
import type { ColorMode } from "../theme/useColorTheme";

export function ThemeToggle({ mode, onToggle }: { mode: ColorMode; onToggle: () => void }) {
  return (
    <button className="nav-icon-button" onClick={onToggle} type="button">
      {mode === "dark" ? "Light" : "Dark"}
    </button>
  );
}

export function ToastStack({ error, notice, onDismiss }: { error: string | null; notice: string | null; onDismiss: () => void }) {
  if (!error && !notice) return null;

  return (
    <div className="toast-stack" aria-live="polite">
      {error ? (
        <div className="toast toast--error error-copy" role="alert">
          <span>{error}</span>
          <button onClick={onDismiss} type="button">Dismiss</button>
        </div>
      ) : null}
      {notice ? (
        <div className="toast toast--success success-copy" role="status">
          <span>{notice}</span>
          <button onClick={onDismiss} type="button">Dismiss</button>
        </div>
      ) : null}
    </div>
  );
}

export function DialogFrame({
  children,
  description,
  onOpenChange,
  open,
  title,
}: {
  children: ReactNode;
  description: string;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  title: string;
}) {
  useOverlayLifecycle(open, onOpenChange);
  if (!open) return null;

  return (
    <div className="dialog-overlay" onMouseDown={() => onOpenChange(false)} role="presentation">
      <section
        aria-describedby="dialog-description"
        aria-label={title}
        aria-modal="true"
        className="dialog-panel"
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
      >
        <p className="sr-only" id="dialog-description">{description}</p>
        {children}
      </section>
    </div>
  );
}

export function SheetFrame({
  children,
  description,
  onOpenChange,
  open,
  title,
}: {
  children: ReactNode;
  description: string;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  title: string;
}) {
  useOverlayLifecycle(open, onOpenChange);
  if (!open) return null;

  return (
    <div className="sheet-overlay" onMouseDown={() => onOpenChange(false)} role="presentation">
      <aside
        aria-describedby="sheet-description"
        aria-label={title}
        aria-modal="true"
        className="sheet-panel"
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
      >
        <p className="sr-only" id="sheet-description">{description}</p>
        {children}
      </aside>
    </div>
  );
}

function useOverlayLifecycle(open: boolean, onOpenChange: (open: boolean) => void) {
  useEffect(() => {
    if (!open) return;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onOpenChange(false);
    }
    document.addEventListener("keydown", handleKeyDown);
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      document.body.style.overflow = "";
    };
  }, [onOpenChange, open]);
}

export function PreferenceRow({ label, mono, value }: { label: string; mono?: boolean; value: string }) {
  return (
    <div className="preference-row">
      <span>{label}</span>
      <strong className={mono ? "mono" : undefined}>{value}</strong>
    </div>
  );
}

export function SegmentedControl({
  label,
  onChange,
  options,
  value,
}: {
  label: string;
  onChange: (value: string) => void;
  options: string[];
  value: string;
}) {
  return (
    <div className="segmented-control" aria-label={label} role="group">
      {options.map((option) => (
        <button
          aria-pressed={option === value}
          className={option === value ? "segmented-control__item segmented-control__item--active" : "segmented-control__item"}
          key={option}
          onClick={() => onChange(option)}
          type="button"
        >
          {option}
        </button>
      ))}
    </div>
  );
}

export function SummaryTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="summary-tile">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

export function LoadingScreen() {
  return (
    <main aria-busy="true" aria-live="polite" className="loading-screen" role="status">
      <div className="loading-orb" />
      <p className="eyebrow">Accounts Repo</p>
      <h1>Loading review pack...</h1>
    </main>
  );
}

export function StatusPill({ status }: { status: ReviewStatus }) {
  const label = reviewStatusLabel(status);
  return (
    <span aria-label={`Review status: ${label}`} className={`status-pill status-pill--${status}`}>
      {label}
    </span>
  );
}

export function KeyValue({ label, mono, value }: { label: string; mono?: boolean; value: string }) {
  return (
    <div className="key-value">
      <span>{label}</span>
      <strong className={mono ? "mono" : undefined}>{value}</strong>
    </div>
  );
}
