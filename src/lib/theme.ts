export type Theme = "light" | "dark" | "system";

const THEME_STORAGE_KEY = "opets-theme";

export const THEME_OPTIONS: { value: Theme; label: string }[] = [
  { value: "light", label: "Claro" },
  { value: "dark", label: "Escuro" },
  { value: "system", label: "Sistema" },
];

const systemPrefersDark = () =>
  typeof window !== "undefined" &&
  window.matchMedia("(prefers-color-scheme: dark)").matches;

export const getThemePreference = (): Theme => {
  try {
    const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
    if (stored === "light" || stored === "dark" || stored === "system") {
      return stored;
    }
  } catch {
    // The default theme still applies when storage is unavailable.
  }
  return "system";
};

export const applyTheme = (theme: Theme) => {
  const resolved =
    theme === "system" ? (systemPrefersDark() ? "dark" : "light") : theme;
  document.documentElement.classList.toggle("dark", resolved === "dark");
};

export const setThemePreference = (theme: Theme) => {
  applyTheme(theme);
  try {
    window.localStorage.setItem(THEME_STORAGE_KEY, theme);
  } catch {
    // The selected theme still applies for this session when storage is unavailable.
  }
};

let systemThemeQuery: MediaQueryList | null = null;

export const watchSystemTheme = () => {
  if (systemThemeQuery || typeof window === "undefined") {
    return;
  }
  systemThemeQuery = window.matchMedia("(prefers-color-scheme: dark)");
  systemThemeQuery.addEventListener("change", () => {
    if (getThemePreference() === "system") {
      applyTheme("system");
    }
  });
};