import { THEMES, getTheme, setTheme, type ThemeName } from "./theme";
import { updateAllTerminalThemes } from "./terminal";

function renderThemeGrid(): void {
  const grid = document.querySelector<HTMLDivElement>("#theme-grid")!;
  grid.innerHTML = "";
  const current = getTheme();

  for (const t of THEMES) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "theme-option" + (t.value === current ? " active" : "");
    btn.innerHTML = `<span class="theme-swatch theme-swatch-${t.value}"></span>${t.label}`;
    btn.addEventListener("click", () => {
      setTheme(t.value as ThemeName);
      updateAllTerminalThemes(t.value as ThemeName);
      renderThemeGrid();
    });
    grid.appendChild(btn);
  }
}

function openSettings(): void {
  renderThemeGrid();
  document.querySelector<HTMLDivElement>("#settings-overlay")!.style.display = "flex";
}

function closeSettings(): void {
  document.querySelector<HTMLDivElement>("#settings-overlay")!.style.display = "none";
}

window.addEventListener("DOMContentLoaded", () => {
  document.getElementById("settings-btn")?.addEventListener("click", openSettings);
  document.getElementById("settings-close")?.addEventListener("click", closeSettings);
  document.getElementById("settings-overlay")?.addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeSettings();
  });
});
