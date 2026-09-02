import { describe, expect, it } from "vitest";
import { niceSwrMax } from "./plot";

describe("niceSwrMax", () => {
  it("snaps a well-matched trace to a tight ceiling", () => {
    expect(niceSwrMax(1.08)).toBe(1.5);
    expect(niceSwrMax(1.5)).toBe(1.5);
  });

  it("steps up through the nice values", () => {
    expect(niceSwrMax(1.6)).toBe(2);
    expect(niceSwrMax(2.4)).toBe(3);
    expect(niceSwrMax(4.9)).toBe(5);
    expect(niceSwrMax(19.74)).toBe(20);
  });

  it("rounds far-out values up to the next 100", () => {
    expect(niceSwrMax(140)).toBe(200);
    expect(niceSwrMax(999)).toBe(1000);
  });
});
