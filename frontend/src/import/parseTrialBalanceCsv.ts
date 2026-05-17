import type { ImportTrialBalanceLine } from "../types";

export function parseTrialBalanceCsv(csvText: string): ImportTrialBalanceLine[] {
  const rows = parseCsv(csvText).filter((row) => row.some((cell) => cell.trim() !== ""));
  if (rows.length < 2) {
    throw new Error("CSV must include a header row and at least one trial balance line");
  }

  const headers = rows[0].map((header) => header.trim().toLowerCase());
  const requiredHeaders = ["account_code", "account_name", "account_type", "amount", "fs_line", "assertion"];
  for (const header of requiredHeaders) {
    if (!headers.includes(header)) throw new Error(`CSV is missing required column: ${header}`);
  }

  return rows.slice(1).map((row, index) => {
    const value = (header: string) => row[headers.indexOf(header)]?.trim() ?? "";

    return {
      account_code: value("account_code"),
      account_name: value("account_name"),
      account_type: parseAccountType(value("account_type"), index + 2),
      amount: value("amount"),
      fs_line: value("fs_line"),
      assertion: value("assertion"),
    };
  });
}

function parseAccountType(value: string, rowNumber: number): ImportTrialBalanceLine["account_type"] {
  const normalized = value.trim().toLowerCase().replaceAll(" ", "_");
  if (
    normalized === "asset" ||
    normalized === "liability" ||
    normalized === "equity" ||
    normalized === "income" ||
    normalized === "expense"
  ) {
    return normalized;
  }

  throw new Error(`Invalid account_type on row ${rowNumber}: ${value}`);
}

function parseCsv(input: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = "";
  let inQuotes = false;

  for (let index = 0; index < input.length; index += 1) {
    const char = input[index];
    const nextChar = input[index + 1];

    if (char === '"') {
      if (inQuotes && nextChar === '"') {
        cell += '"';
        index += 1;
      } else {
        inQuotes = !inQuotes;
      }
      continue;
    }

    if (char === "," && !inQuotes) {
      row.push(cell);
      cell = "";
      continue;
    }

    if ((char === "\n" || char === "\r") && !inQuotes) {
      if (char === "\r" && nextChar === "\n") index += 1;
      row.push(cell);
      rows.push(row);
      row = [];
      cell = "";
      continue;
    }

    cell += char;
  }

  row.push(cell);
  rows.push(row);

  return rows;
}
