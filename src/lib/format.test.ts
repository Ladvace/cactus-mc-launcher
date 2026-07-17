import { describe, it, expect } from "vitest";
import { formatCount } from "./format";

describe("formatCount", () => {
  it("leaves values under 1000 as-is", () => {
    expect(formatCount(0)).toBe("0");
    expect(formatCount(999)).toBe("999");
  });
  it("formats thousands with one decimal below 10k", () => {
    expect(formatCount(1234)).toBe("1.2K");
    expect(formatCount(9999)).toBe("10.0K");
  });
  it("drops the decimal at 10k+", () => {
    expect(formatCount(12000)).toBe("12K");
  });
  it("formats millions", () => {
    expect(formatCount(3_400_000)).toBe("3.4M");
  });
});
