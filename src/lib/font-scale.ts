export type FontScale = "sm" | "md" | "lg";

const FONT_SCALE_STORAGE_KEY = "opets-font-scale";

export const FONT_SCALE_OPTIONS: { value: FontScale; scale: number; label: string }[] = [
  { value: "sm", scale: 0.9, label: "Pequena" },
  { value: "md", scale: 1, label: "Padrão" },
  { value: "lg", scale: 1.1, label: "Grande" },
];

export const getFontScalePreference = (): FontScale => {
  try {
    const stored = window.localStorage.getItem(FONT_SCALE_STORAGE_KEY);
    if (stored === "sm" || stored === "md" || stored === "lg") {
      return stored;
    }
  } catch {
    // The default scale still applies when storage is unavailable.
  }
  return "md";
};

export const applyFontScale = (scale: FontScale) => {
  const option =
    FONT_SCALE_OPTIONS.find((candidate) => candidate.value === scale) ??
    FONT_SCALE_OPTIONS[1];
  document.documentElement.style.setProperty("--font-scale", String(option.scale));
};

export const setFontScalePreference = (scale: FontScale) => {
  applyFontScale(scale);
  try {
    window.localStorage.setItem(FONT_SCALE_STORAGE_KEY, scale);
  } catch {
    // The selected scale still applies for this session when storage is unavailable.
  }
};