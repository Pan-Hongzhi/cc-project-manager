import { describe, expect, it } from "vitest";
import { categoryKey, formatBytes, formatRelative, formatTokens } from "./format";

describe("formatBytes", () => {
  it("uses 1024 base with one decimal above KB", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(2 * 1024 * 1024)).toBe("2.0 MB");
    expect(formatBytes(1.2 * 1024 ** 3)).toBe("1.2 GB");
  });
});

describe("formatRelative", () => {
  const now = 1_800_000_000_000;
  it("buckets by minutes/hours/days/months", () => {
    expect(formatRelative(null, now)).toBe("—");
    expect(formatRelative(now - 30_000, now)).toBe("刚刚");
    expect(formatRelative(now - 5 * 60_000, now)).toBe("5 分钟前");
    expect(formatRelative(now - 3 * 3_600_000, now)).toBe("3 小时前");
    expect(formatRelative(now - 12 * 86_400_000, now)).toBe("12 天前");
    expect(formatRelative(now - 130 * 86_400_000, now)).toBe("4 个月前");
  });
});

describe("formatTokens", () => {
  it("groups thousands and abbreviates millions", () => {
    expect(formatTokens(999)).toBe("999");
    expect(formatTokens(12345)).toBe("12,345");
    expect(formatTokens(12_345_678)).toBe("12.3M");
  });
});

describe("categoryKey", () => {
  it("joins kind and name", () => {
    expect(categoryKey({ kind: "transcripts" })).toBe("transcripts");
    expect(categoryKey({ kind: "legacy", name: "todos" })).toBe("legacy:todos");
  });
});
