import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";
import { applyTheme, getThemePreference, watchSystemTheme } from "./lib/theme";
import { applyFontScale, getFontScalePreference } from "./lib/font-scale";

applyTheme(getThemePreference());
watchSystemTheme();
applyFontScale(getFontScalePreference());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
