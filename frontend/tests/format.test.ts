import { describe, expect, it } from "vitest";
import { formatSignedCurrency, reviewStatusLabel } from "../src/format";

describe("financial formatting", () => {
  it("ensures positive financial diffs keep an explicit plus sign for reviewers", () => {
    expect(formatSignedCurrency("3900.00")).toMatch(/^\+RM\s?3,900\.00$/);
  });

  it("prevents raw review status codes leaking into client sign-off copy", () => {
    expect(reviewStatusLabel("reviewer_approved")).toBe("Reviewer approved");
  });
});
