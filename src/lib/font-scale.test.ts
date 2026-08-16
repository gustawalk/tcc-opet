import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  FONT_SCALE_OPTIONS,
  applyFontScale,
  getFontScalePreference,
  setFontScalePreference,
} from "@/lib/font-scale";

beforeEach(() => {
  window.localStorage.clear();
  document.documentElement.style.removeProperty("--font-scale");
});

describe("font scale preference", () => {
  it("defaults to the standard scale", () => {
    expect(getFontScalePreference()).toBe("md");
  });

  it("returns the stored preference", () => {
    window.localStorage.setItem("opets-font-scale", "lg");
    expect(getFontScalePreference()).toBe("lg");
  });

  it("applies the css variable for each option", () => {
    for (const option of FONT_SCALE_OPTIONS) {
      applyFontScale(option.value);
      expect(
        document.documentElement.style.getPropertyValue("--font-scale"),
      ).toBe(String(option.scale));
    }
  });

  it("persists and applies the selected scale", () => {
    setFontScalePreference("lg");
    expect(window.localStorage.getItem("opets-font-scale")).toBe("lg");
    expect(
      document.documentElement.style.getPropertyValue("--font-scale"),
    ).toBe("1.1");
  });

  it("falls back to the standard scale when storage is unavailable", () => {
    const getItem = vi
      .spyOn(window.localStorage, "getItem")
      .mockImplementation(() => {
        throw new Error("storage unavailable");
      });
    expect(getFontScalePreference()).toBe("md");
    getItem.mockRestore();
  });
});