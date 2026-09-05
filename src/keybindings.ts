export interface KeyCombo {
  key: string;
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  meta: boolean;
}

export interface Keybindings {
  copy: KeyCombo;
  paste: KeyCombo;
}

export type KeybindAction = keyof Keybindings;

const STORAGE_KEY = "keybindings";

const DEFAULT_BINDINGS: Keybindings = {
  copy: { key: "c", ctrl: false, shift: true, alt: false, meta: false },
  paste: { key: "v", ctrl: false, shift: true, alt: false, meta: false },
};

const MODIFIER_KEYS = new Set(["Shift", "Control", "Alt", "Meta", "OS"]);

function normalizeKey(key: string): string {
  return key.length === 1 ? key.toLowerCase() : key;
}

export function getKeybindings(): Keybindings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_BINDINGS;
    const parsed = JSON.parse(raw) as Partial<Keybindings>;
    return {
      copy: parsed.copy ?? DEFAULT_BINDINGS.copy,
      paste: parsed.paste ?? DEFAULT_BINDINGS.paste,
    };
  } catch {
    return DEFAULT_BINDINGS;
  }
}

export function setKeybinding(action: KeybindAction, combo: KeyCombo): Keybindings {
  const updated = { ...getKeybindings(), [action]: combo };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
  return updated;
}

/** True while a keydown is still just a bare modifier press (e.g. only Shift so far). */
export function isBareModifier(event: KeyboardEvent): boolean {
  return MODIFIER_KEYS.has(event.key);
}

export function hasModifier(combo: KeyCombo): boolean {
  return combo.ctrl || combo.shift || combo.alt || combo.meta;
}

export function comboFromEvent(event: KeyboardEvent): KeyCombo {
  return {
    key: normalizeKey(event.key),
    ctrl: event.ctrlKey,
    shift: event.shiftKey,
    alt: event.altKey,
    meta: event.metaKey,
  };
}

export function combosEqual(a: KeyCombo, b: KeyCombo): boolean {
  return a.key === b.key && a.ctrl === b.ctrl && a.shift === b.shift && a.alt === b.alt && a.meta === b.meta;
}

export function matchesCombo(event: KeyboardEvent, combo: KeyCombo): boolean {
  return combosEqual(comboFromEvent(event), combo);
}

const SPECIAL_KEY_LABELS: Record<string, string> = {
  " ": "Space",
  arrowup: "↑",
  arrowdown: "↓",
  arrowleft: "←",
  arrowright: "→",
  escape: "Esc",
};

export function formatCombo(combo: KeyCombo): string {
  const parts: string[] = [];
  if (combo.ctrl) parts.push("Ctrl");
  if (combo.alt) parts.push("Alt");
  if (combo.shift) parts.push("Shift");
  if (combo.meta) parts.push("Win");

  const lower = combo.key.toLowerCase();
  const label = SPECIAL_KEY_LABELS[lower] ?? (combo.key.length === 1 ? combo.key.toUpperCase() : combo.key);
  parts.push(label);

  return parts.join("+");
}
