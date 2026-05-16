import { expect, test } from "@playwright/test";

async function ensureWorkspaceImported(page: import("@playwright/test").Page) {
  await page.goto("/");

  const importHeading = page.getByRole("heading", { name: /Import a mapped trial balance/ });
  await importHeading.waitFor({ state: "visible", timeout: 5_000 }).catch(() => undefined);
  if (!(await importHeading.isVisible().catch(() => false))) return;

  await page.getByLabel("Entity name").fill("Nusantara Precision Sdn Bhd");
  await page.getByLabel("Registration number").fill("202001034561 (1390882-X)");
  await page.getByLabel("Owner", { exact: true }).fill("Hazli Johar");
  await page.getByLabel("Owner email").fill("hazli@nusantara.test");
  await page.getByLabel("Firm").fill("Amjad & Hazli Advisory");
  await page.getByLabel("Preparer", { exact: true }).fill("Aina Rahman");
  await page.getByLabel("Reviewer", { exact: true }).fill("Amjad Salleh");
  await page.getByLabel("Reviewer email").fill("aina@ahadvisory.test");
  await page.getByLabel("Client signer", { exact: true }).fill("Hazli Johar");
  await page.getByLabel("Client signer email").fill("aina@ahadvisory.test");
  await page.getByLabel("Branch label").fill("FY2026 Year-End");
  await page.getByLabel("Period start").fill("2025-07-01");
  await page.getByLabel("Period end").fill("2026-06-30");
  await page.getByLabel("Source label").fill("Real TB export 2026-06-30");
  await page.getByLabel("CSV contents").fill([
    "account_code,account_name,account_type,amount,fs_line,assertion",
    "1000,Cash at Bank,asset,245000.00,Cash and Bank,Existence",
    "1100,Trade Receivables,asset,183500.00,Trade Receivables,Recoverability",
    "1200,Inventories,asset,92000.00,Inventories,Valuation",
    "1500,Plant and Equipment,asset,380000.00,\"Property, Plant and Equipment\",Existence",
    "1600,Accumulated Depreciation,asset,-152000.00,Accumulated Depreciation,Valuation",
    "2000,Trade Payables,liability,-121000.00,Trade Payables,Completeness",
    "2100,Accruals,liability,-68000.00,Accruals,Completeness",
    "2200,Tax Payable,liability,-34000.00,Tax Payable,Accuracy",
    "3000,Share Capital,equity,-250000.00,Share Capital,Rights and obligations",
    "3100,Retained Earnings,equity,-175400.00,Retained Earnings,Accuracy",
    "4000,Revenue,income,-1350000.00,Revenue,Completeness",
    "5000,Cost of Sales,expense,702000.00,Cost of Sales,Cut-off",
    "6000,Salaries,expense,286000.00,Administrative Expenses,Accuracy",
    "6100,Rent,expense,84000.00,Administrative Expenses,Cut-off",
    "6200,Professional Fees,expense,42000.00,Administrative Expenses,Cut-off",
    "6300,Depreciation Expense,expense,76000.00,Depreciation,Accuracy",
    "6400,Bank Charges,expense,3900.00,Finance Costs,Accuracy",
    "6500,Tax Expense,expense,56000.00,Tax Expense,Accuracy",
  ].join("\n"));
  await page.getByRole("button", { name: "Import real TB" }).click();
}

test("prevents unsigned accounts by requiring reviewer approval before client signoff", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "mutating sign-off workflow runs once against the local backend");

  await ensureWorkspaceImported(page);
  page.on("dialog", (dialog) => dialog.accept());

  await expect(page.getByRole("heading", { name: "Nusantara Precision Sdn Bhd" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign as client" })).toHaveCount(0);

  await page.getByRole("button", { name: "Approve as reviewer" }).click();
  await expect(page.getByText("Reviewer approved").first()).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign as client" })).toBeVisible();

  await page.getByRole("button", { name: "Sign as client" }).click();
  await expect(page.getByText("Signed and frozen").first()).toBeVisible();
  await expect(page.getByText("Signed branches are immutable.")).toBeVisible();
  await page.getByRole("tab", { name: /Audit/ }).click();
  await expect(page.getByText("Client director signed the review pack")).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign as client" })).toHaveCount(0);
});

test("keeps the review workspace readable on a mobile viewport", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "mobile-chrome", "mobile project owns responsive coverage");

  await ensureWorkspaceImported(page);

  await expect(page.getByRole("heading", { name: "Nusantara Precision Sdn Bhd" })).toBeVisible();
  await expect(page.getByLabel("Financial summary")).toBeVisible();
  await expect(page.getByLabel("Review workspace tabs")).toBeVisible();
  await expect(page.getByRole("button", { name: /Nusantara Precision Sdn Bhd/ })).toBeVisible();
});
