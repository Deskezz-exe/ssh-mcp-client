export type ThemeName = "dark" | "light" | "hacker" | "claude";

export const THEMES: { value: ThemeName; label: string }[] = [
  { value: "dark", label: "Тёмная" },
  { value: "light", label: "Светлая" },
  { value: "hacker", label: "Хакерская" },
  { value: "claude", label: "Claude Code" },
];

const STORAGE_KEY = "theme";
const DEFAULT_THEME: ThemeName = "dark";

interface XtermTheme {
  background: string;
  foreground: string;
  cursor: string;
  selectionBackground: string;
}

const XTERM_THEMES: Record<ThemeName, XtermTheme> = {
  dark: { background: "#181818", foreground: "#e6e6e6", cursor: "#e6e6e6", selectionBackground: "#3568c955" },
  light: { background: "#ffffff", foreground: "#1a1a1e", cursor: "#1a1a1e", selectionBackground: "#2f5fc433" },
  hacker: { background: "#000000", foreground: "#33ff33", cursor: "#33ff33", selectionBackground: "#33ff3344" },
  claude: { background: "#1e1a17", foreground: "#f0e6df", cursor: "#d97757", selectionBackground: "#d9775744" },
};

function isThemeName(value: string | null): value is ThemeName {
  return value === "dark" || value === "light" || value === "hacker" || value === "claude";
}

export function getTheme(): ThemeName {
  const stored = localStorage.getItem(STORAGE_KEY);
  return isThemeName(stored) ? stored : DEFAULT_THEME;
}

export function applyTheme(theme: ThemeName): void {
  document.documentElement.setAttribute("data-theme", theme);
}

export function setTheme(theme: ThemeName): void {
  localStorage.setItem(STORAGE_KEY, theme);
  applyTheme(theme);
}

export function xtermThemeFor(theme: ThemeName): XtermTheme {
  return XTERM_THEMES[theme];
}
