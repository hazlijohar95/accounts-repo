import { expect, test } from "@playwright/test";

test("prevents unsigned accounts by requiring reviewer approval before client signoff", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "mutating sign-off workflow runs once against the seeded backend");

  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Nusantara Precision Sdn Bhd" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign as client" })).toBeDisabled();

  await page.getByRole("button", { name: "Approve as reviewer" }).click();
  await expect(page.getByText("Reviewer approved").first()).toBeVisible();

  await page.getByRole("button", { name: "Sign as client" }).click();
  await expect(page.getByText("Signed and frozen").first()).toBeVisible();
  await expect(page.getByText("Client director signed the review pack")).toBeVisible();
  await expect(page.getByRole("button", { name: "Branch frozen after sign-off" })).toBeDisabled();
});

test("keeps the review workspace readable on a mobile viewport", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "mobile-chrome", "mobile project owns responsive coverage");

  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Nusantara Precision Sdn Bhd" })).toBeVisible();
  await expect(page.getByLabel("Financial summary")).toBeVisible();
  await expect(page.getByRole("button", { name: /Nusantara Precision Sdn Bhd/ })).toBeVisible();
});
