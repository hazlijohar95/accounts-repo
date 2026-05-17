import { describe, expect, it } from "vitest";
import * as XLSX from "xlsx";
import { parseTrialBalanceCsv, parseTrialBalanceFile } from "../src/import/parseTrialBalanceCsv";

describe("parseTrialBalanceCsv", () => {
  it("parses mapped trial balance rows with quoted commas", () => {
    expect(
      parseTrialBalanceCsv(
        'account_code,account_name,account_type,amount,fs_line,assertion\n1000,"Cash, Bank",asset,1000.00,Cash and Bank,Existence\n4000,Revenue,income,-1000.00,Revenue,Completeness',
      ),
    ).toEqual([
      {
        account_code: "1000",
        account_name: "Cash, Bank",
        account_type: "asset",
        amount: "1000.00",
        fs_line: "Cash and Bank",
        assertion: "Existence",
      },
      {
        account_code: "4000",
        account_name: "Revenue",
        account_type: "income",
        amount: "-1000.00",
        fs_line: "Revenue",
        assertion: "Completeness",
      },
    ]);
  });

  it("rejects unknown account types before import", () => {
    expect(() =>
      parseTrialBalanceCsv(
        "account_code,account_name,account_type,amount,fs_line,assertion\n1000,Cash,bank,1000.00,Cash and Bank,Existence",
      ),
    ).toThrow("Invalid account_type on row 2: bank");
  });

  it("parses XLSX workbooks and returns source evidence metadata", async () => {
    const workbook = XLSX.utils.book_new();
    const sheet = XLSX.utils.aoa_to_sheet([
      ["account_code", "account_name", "account_type", "amount", "fs_line", "assertion"],
      ["1000", "Cash at Bank", "asset", "1000.00", "Cash and Bank", "Existence"],
      ["4000", "Revenue", "income", "-1000.00", "Revenue", "Completeness"],
    ]);
    XLSX.utils.book_append_sheet(workbook, sheet, "TB");
    const bytes = XLSX.write(workbook, { bookType: "xlsx", type: "array" }) as ArrayBuffer;
    const file = new File([bytes], "tb.xlsx");

    const parsed = await parseTrialBalanceFile(file);

    expect(parsed.parser).toBe("xlsx");
    expect(parsed.fileHash).toHaveLength(64);
    expect(parsed.rowCount).toBe(2);
    expect(parsed.trialBalance[0]).toMatchObject({
      account_code: "1000",
      fs_line: "Cash and Bank",
    });
  });
});
