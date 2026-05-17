import { useState } from "react";
import type { ChangeEvent, FormEvent } from "react";
import type { ImportWorkspacePayload } from "../types";
import { hashTrialBalanceText, parseTrialBalanceCsv, parseTrialBalanceFile } from "./parseTrialBalanceCsv";

export function ImportEmptyState({
  currentUser,
  error,
  importing,
  onImport,
  onRetry,
}: {
  currentUser: { name: string; email: string };
  error: string | null;
  importing: boolean;
  onImport: (payload: ImportWorkspacePayload) => void;
  onRetry: () => void;
}) {
  return (
    <main className="empty-state empty-state--import">
      <section className="import-intro">
        <p className="eyebrow">Accounts Repo</p>
        <h1>Import a mapped trial balance to open a review repo.</h1>
        <p className="empty-copy">
          Start with source data you can trace. The import preview will become the first commit,
          then reviewers can approve and clients can sign a locked evidence pack.
        </p>
        {error ? <p className="error-copy" role="alert">{error}</p> : null}
        {error ? (
          <button className="secondary-button" onClick={onRetry} type="button">
            Retry API connection
          </button>
        ) : null}
      </section>

      <ImportWorkspaceForm currentUser={currentUser} importing={importing} onImport={onImport} />
    </main>
  );
}

function ImportWorkspaceForm({
  currentUser,
  importing,
  onImport,
}: {
  currentUser: { name: string; email: string };
  importing: boolean;
  onImport: (payload: ImportWorkspacePayload) => void;
}) {
  const [entityName, setEntityName] = useState("");
  const [registrationNumber, setRegistrationNumber] = useState("");
  const [jurisdiction, setJurisdiction] = useState("Malaysia");
  const [entityType, setEntityType] = useState("Sdn Bhd");
  const [ownerName, setOwnerName] = useState("");
  const [ownerEmail, setOwnerEmail] = useState("");
  const [firmName, setFirmName] = useState("Amjad & Hazli Advisory");
  const [preparerName, setPreparerName] = useState(currentUser.name);
  const [preparerEmail, setPreparerEmail] = useState(currentUser.email);
  const [reviewerName, setReviewerName] = useState("");
  const [reviewerEmail, setReviewerEmail] = useState("");
  const [clientSignerName, setClientSignerName] = useState("");
  const [clientSignerEmail, setClientSignerEmail] = useState("");
  const [branchLabel, setBranchLabel] = useState("");
  const [periodStart, setPeriodStart] = useState("");
  const [periodEnd, setPeriodEnd] = useState("");
  const [sourceLabel, setSourceLabel] = useState("");
  const [csvText, setCsvText] = useState("");
  const [sourceFileName, setSourceFileName] = useState<string | null>(null);
  const [sourceFileHash, setSourceFileHash] = useState<string | null>(null);
  const [sourceParser, setSourceParser] = useState<"csv">("csv");
  const [sourceRowCount, setSourceRowCount] = useState<number | null>(null);
  const [parseError, setParseError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    try {
      const custodyError = custodyEmailError({
        currentUserEmail: currentUser.email,
        ownerEmail,
        preparerEmail,
        reviewerEmail,
        signerEmail: clientSignerEmail,
      });
      if (custodyError) {
        setParseError(custodyError);
        return;
      }

      const trialBalance = parseTrialBalanceCsv(csvText);
      const fileHash = sourceFileHash ?? await hashTrialBalanceText(csvText);
      setParseError(null);
      onImport({
        entity_name: entityName,
        registration_number: registrationNumber,
        jurisdiction,
        entity_type: entityType,
        owner_name: ownerName,
        owner_email: ownerEmail,
        firm_name: firmName,
        preparer_name: preparerName,
        preparer_email: preparerEmail,
        reviewer_name: reviewerName,
        reviewer_email: reviewerEmail,
        client_signer_name: clientSignerName,
        client_signer_email: clientSignerEmail,
        branch_label: branchLabel,
        period_start: periodStart,
        period_end: periodEnd,
        source_label: sourceLabel,
        source_file_name: sourceFileName,
        source_file_hash: fileHash,
        source_parser: sourceParser,
        source_row_count: sourceRowCount ?? trialBalance.length,
        trial_balance: trialBalance,
      });
    } catch (caught) {
      setParseError(caught instanceof Error ? caught.message : "Could not parse CSV");
    }
  }

  async function handleFileSelect(event: ChangeEvent<HTMLInputElement>) {
    const file = event.currentTarget.files?.[0];
    if (!file) return;

    try {
      const parsedFile = await parseTrialBalanceFile(file);
      setSourceLabel((current) => current || parsedFile.fileName);
      setCsvText(parsedFile.csvText);
      setSourceFileName(parsedFile.fileName);
      setSourceFileHash(parsedFile.fileHash);
      setSourceParser(parsedFile.parser);
      setSourceRowCount(parsedFile.rowCount);
      setParseError(null);
    } catch (caught) {
      setParseError(caught instanceof Error ? caught.message : "Could not parse selected file");
    }
  }

  function handleCsvTextChange(value: string) {
    setCsvText(value);
    setSourceFileName(null);
    setSourceFileHash(null);
    setSourceParser("csv");
    setSourceRowCount(null);
  }

  return (
    <form className="import-panel" onSubmit={handleSubmit}>
      <div>
        <p className="section-label">Source data import</p>
        <h2>Mapped trial balance</h2>
        <p>
          Required CSV columns: <code>account_code</code>, <code>account_name</code>, <code>account_type</code>, <code>amount</code>, <code>fs_line</code>, <code>assertion</code>.
        </p>
      </div>

      <div className="form-grid">
        <label>
          Entity name
          <input required value={entityName} onChange={(event) => setEntityName(event.target.value)} />
        </label>
        <label>
          Registration number
          <input required value={registrationNumber} onChange={(event) => setRegistrationNumber(event.target.value)} />
        </label>
        <label>
          Jurisdiction
          <input required value={jurisdiction} onChange={(event) => setJurisdiction(event.target.value)} />
        </label>
        <label>
          Entity type
          <input required value={entityType} onChange={(event) => setEntityType(event.target.value)} />
        </label>
        <label>
          Owner
          <input required value={ownerName} onChange={(event) => setOwnerName(event.target.value)} />
        </label>
        <label>
          Owner email
          <input required type="email" value={ownerEmail} onChange={(event) => setOwnerEmail(event.target.value)} />
        </label>
        <label>
          Firm
          <input required value={firmName} onChange={(event) => setFirmName(event.target.value)} />
        </label>
        <label>
          Preparer
          <input required value={preparerName} onChange={(event) => setPreparerName(event.target.value)} />
        </label>
        <label>
          Preparer email
          <input required type="email" value={preparerEmail} onChange={(event) => setPreparerEmail(event.target.value)} />
        </label>
        <label>
          Reviewer
          <input required value={reviewerName} onChange={(event) => setReviewerName(event.target.value)} />
        </label>
        <label>
          Reviewer email
          <input required type="email" value={reviewerEmail} onChange={(event) => setReviewerEmail(event.target.value)} />
        </label>
        <label>
          Client signer
          <input required value={clientSignerName} onChange={(event) => setClientSignerName(event.target.value)} />
        </label>
        <label>
          Client signer email
          <input required type="email" value={clientSignerEmail} onChange={(event) => setClientSignerEmail(event.target.value)} />
        </label>
        <label>
          Branch label
          <input required value={branchLabel} onChange={(event) => setBranchLabel(event.target.value)} />
        </label>
        <label>
          Period start
          <input required type="date" value={periodStart} onChange={(event) => setPeriodStart(event.target.value)} />
        </label>
        <label>
          Period end
          <input required type="date" value={periodEnd} onChange={(event) => setPeriodEnd(event.target.value)} />
        </label>
      </div>

      <label>
        Source label
        <input required value={sourceLabel} onChange={(event) => setSourceLabel(event.target.value)} />
      </label>

      <label>
        Source CSV file
        <input accept=".csv,text/csv" type="file" onChange={(event) => void handleFileSelect(event)} />
      </label>

      {sourceFileHash ? (
        <p className="source-evidence">
          Parsed {sourceRowCount ?? 0} rows from {sourceFileName} with {sourceParser.toUpperCase()} parser. Source hash {sourceFileHash.slice(0, 16)}...
        </p>
      ) : null}

      <label>
        CSV contents
        <textarea required rows={10} value={csvText} onChange={(event) => handleCsvTextChange(event.target.value)} />
      </label>

      {parseError ? <p className="error-copy" role="alert">{parseError}</p> : null}

      <button className="primary-button" disabled={importing} type="submit">
        {importing ? "Importing..." : "Import real TB"}
      </button>
    </form>
  );
}

function custodyEmailError({
  currentUserEmail,
  ownerEmail,
  preparerEmail,
  reviewerEmail,
  signerEmail,
}: {
  currentUserEmail: string;
  ownerEmail: string;
  preparerEmail: string;
  reviewerEmail: string;
  signerEmail: string;
}) {
  const current = normalizeEmail(currentUserEmail);
  const owner = normalizeEmail(ownerEmail);
  const preparer = normalizeEmail(preparerEmail);
  const reviewer = normalizeEmail(reviewerEmail);
  const signer = normalizeEmail(signerEmail);

  if (preparer !== current) {
    return "Preparer email must match your signed-in email.";
  }

  if (preparer === reviewer || preparer === signer || reviewer === signer || owner === preparer || owner === reviewer) {
    return "Custody role emails must be distinct. Owner email may only match the client signer.";
  }

  return null;
}

function normalizeEmail(email: string) {
  return email.trim().toLowerCase();
}
