import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  THEME_OPTIONS,
  applyTheme,
  getThemePreference,
  setThemePreference,
} from "@/lib/theme";

const originalMatchMedia = window.matchMedia;

function installMatchMedia(matches: boolean) {
  const listeners = new Set<() => void>();
  const query = {
    matches,
    media: "(prefers-color-scheme: dark)",
    onchange: null,
    addEventListener: (_event: string, handler: () => void) => {
      listeners.add(handler);
    },
    removeEventListener: (_event: string, handler: () => void) => {
      listeners.delete(handler);
    },
    dispatchEvent: () => true,
  };
  window.matchMedia = vi.fn().mockReturnValue(
    query as unknown as MediaQueryList,
  );
  return {
    setMatches: (value: boolean) => {
      query.matches = value;
    },
    listeners,
  };
}

beforeEach(() => {
  window.localStorage.clear();
  document.documentElement.classList.remove("dark");
});

afterEach(() => {
  window.matchMedia = originalMatchMedia;
});

describe("theme preference", () => {
  it("defaults to system when nothing is stored", () => {
    expect(getThemePreference()).toBe("system");
  });

  it("returns the stored preference", () => {
    window.localStorage.setItem("opets-theme", "dark");
    expect(getThemePreference()).toBe("dark");
  });

  it("lists the three theme options", () => {
    expect(THEME_OPTIONS.map((option) => option.value)).toEqual([
      "light",
      "dark",
      "system",
    ]);
  });

  it("applies the light and dark classes", () => {
    applyTheme("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    applyTheme("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("resolves the system theme through prefers-color-scheme", () => {
    installMatchMedia(true);
    applyTheme("system");
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    installMatchMedia(false);
    applyTheme("system");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("persists and applies the selected theme", () => {
    setThemePreference("dark");
    expect(window.localStorage.getItem("opets-theme")).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("falls back to system when storage is unavailable", () => {
    const getItem = vi
      .spyOn(window.localStorage, "getItem")
      .mockImplementation(() => {
        throw new Error("storage unavailable");
      });
    expect(getThemePreference()).toBe("system");
    getItem.mockRestore();
  });
});

describe("system theme watcher", () => {
  it("reapplies the system theme when the OS preference changes", async () => {
    vi.resetModules();
    const module = await import("@/lib/theme");
    const { setMatches, listeners } = installMatchMedia(true);

    module.watchSystemTheme();
    module.setThemePreference("system");
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    setMatches(false);
    for (const listener of listeners) {
      listener();
    }
    expect(document.documentElement.classList.contains("dark")).toBe(false);

    setMatches(true);
    for (const listener of listeners) {
      listener();
    }
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });
});