export interface KeyCombo {
  code: string;
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
  copy: { code: "KeyC", ctrl: false, shift: true, alt: false, meta: false },
  paste: { code: "KeyV", ctrl: false, shift: true, alt: false, meta: false },
};

const MODIFIER_KEYS = new Set(["Shift", "Control", "Alt", "Meta", "OS"]);

function isValidCombo(value: unknown): value is KeyCombo {
  const c = value as Partial<KeyCombo> | null;
  return !!c && typeof c.code === "string" && c.code.length > 0;
}

export function getKeybindings(): Keybindings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_BINDINGS;
    const parsed = JSON.parse(raw) as Partial<Keybindings>;
    return {
      copy: isValidCombo(parsed.copy) ? parsed.copy : DEFAULT_BINDINGS.copy,
      paste: isValidCombo(parsed.paste) ? parsed.paste : DEFAULT_BINDINGS.paste,
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

// event.code is the physical key (e.g. "KeyC"), independent of keyboard
// layout/language — event.key would give a different character on a
// Cyrillic layout for the same physical key, which is why combos used to
// silently fail to match for non-Latin layouts.
export function comboFromEvent(event: KeyboardEvent): KeyCombo {
  return {
    code: event.code,
    ctrl: event.ctrlKey,
    shift: event.shiftKey,
    alt: event.altKey,
    meta: event.metaKey,
  };
}

export function combosEqual(a: KeyCombo, b: KeyCombo): boolean {
  return a.code === b.code && a.ctrl === b.ctrl && a.shift === b.shift && a.alt === b.alt && a.meta === b.meta;
}

export function matchesCombo(event: KeyboardEvent, combo: KeyCombo): boolean {
  return combosEqual(comboFromEvent(event), combo);
}

const SPECIAL_CODE_LABELS: Record<string, string> = {
  Space: "Space",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Escape: "Esc",
};

function labelForCode(code: string): string {
  if (SPECIAL_CODE_LABELS[code]) return SPECIAL_CODE_LABELS[code];
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return code;
}

export function formatCombo(combo: KeyCombo): string {
  const parts: string[] = [];
  if (combo.ctrl) parts.push("Ctrl");
  if (combo.alt) parts.push("Alt");
  if (combo.shift) parts.push("Shift");
  if (combo.meta) parts.push("Win");

  parts.push(labelForCode(combo.code));

  return parts.join("+");
}
