import { THEMES, getTheme, setTheme, type ThemeName } from "./theme";
import { updateAllTerminalThemes } from "./terminal";
import {
  getKeybindings,
  setKeybinding,
  formatCombo,
  comboFromEvent,
  isBareModifier,
  hasModifier,
  combosEqual,
  type KeybindAction,
} from "./keybindings";

const ACTION_LABELS: Record<KeybindAction, string> = {
  copy: "Копировать",
  paste: "Вставить",
};

let listening: KeybindAction | null = null;
let keydownGuard: ((e: KeyboardEvent) => void) | null = null;

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

function renderKeybindList(): void {
  const list = document.querySelector<HTMLDivElement>("#keybind-list")!;
  list.innerHTML = "";
  const bindings = getKeybindings();

  (Object.keys(ACTION_LABELS) as KeybindAction[]).forEach((action) => {
    const row = document.createElement("div");
    row.className = "keybind-row";

    const label = document.createElement("span");
    label.className = "keybind-label";
    label.textContent = ACTION_LABELS[action];
    row.appendChild(label);

    const box = document.createElement("button");
    box.type = "button";
    box.className = "keybind-box";
    box.textContent = listening === action ? "Нажмите комбинацию…" : formatCombo(bindings[action]);
    if (listening === action) box.classList.add("listening");
    box.addEventListener("click", () => startListening(action));
    row.appendChild(box);

    list.appendChild(row);
  });
}

function stopListening(): void {
  if (keydownGuard) {
    window.removeEventListener("keydown", keydownGuard, true);
    keydownGuard = null;
  }
  listening = null;
}

function startListening(action: KeybindAction): void {
  stopListening();
  listening = action;
  renderKeybindList();

  keydownGuard = (event: KeyboardEvent) => {
    event.preventDefault();
    event.stopPropagation();

    if (isBareModifier(event)) return; // wait for a real key on top of the modifier

    if (event.key === "Escape") {
      stopListening();
      renderKeybindList();
      return;
    }

    const combo = comboFromEvent(event);
    if (!hasModifier(combo)) {
      // Refuse combos with no modifier at all — they'd break normal typing.
      return;
    }

    const bindings = getKeybindings();
    const otherAction: KeybindAction = action === "copy" ? "paste" : "copy";
    if (combosEqual(combo, bindings[otherAction])) {
      // Already used by the other action — ignore and keep listening.
      return;
    }

    setKeybinding(action, combo);
    stopListening();
    renderKeybindList();
  };

  window.addEventListener("keydown", keydownGuard, true);
}

function openSettings(): void {
  renderThemeGrid();
  renderKeybindList();
  document.querySelector<HTMLDivElement>("#settings-overlay")!.style.display = "flex";
}

function closeSettings(): void {
  stopListening();
  renderKeybindList();
  document.querySelector<HTMLDivElement>("#settings-overlay")!.style.display = "none";
}

window.addEventListener("DOMContentLoaded", () => {
  document.getElementById("settings-btn")?.addEventListener("click", openSettings);
  document.getElementById("settings-close")?.addEventListener("click", closeSettings);
  document.getElementById("settings-overlay")?.addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeSettings();
  });
});
