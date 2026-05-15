import { expect, test } from "@playwright/test";

test("prevents unsigned accounts by requiring reviewer approval before client signoff", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Nusantara Precision Sdn Bhd" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign as client" })).toBeDisabled();

  await page.getByRole("button", { name: "Approve as reviewer" }).click();
  await expect(page.getByText("Reviewer approved").first()).toBeVisible();

  await page.getByRole("button", { name: "Sign as client" }).click();
  await expect(page.getByText("Signed and frozen").first()).toBeVisible();
  await expect(page.getByText("Client director signed the review pack")).toBeVisible();
});
