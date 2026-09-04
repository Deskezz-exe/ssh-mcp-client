import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface TerminalSession {
  serverId: string;
  term: Terminal;
  fit: FitAddon;
  container: HTMLElement;
  unlistenOutput: UnlistenFn;
  unlistenClosed: UnlistenFn;
}

const sessions = new Map<string, TerminalSession>();

function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

export async function openTerminal(serverId: string, container: HTMLElement): Promise<void> {
  if (sessions.has(serverId)) return;

  const term = new Terminal({
    cursorBlink: true,
    fontFamily: "Consolas, 'Cascadia Mono', monospace",
    fontSize: 14,
    theme: { background: "#181818" },
  });
  const fit = new FitAddon();
  term.loadAddon(fit);
  term.open(container);
  fit.fit();

  // Copy = Shift+C, paste = Shift+V (Ctrl+C/Ctrl+V stay as SIGINT / whatever
  // the remote shell does with them). Both preventDefault() in addition to
  // returning false — without it the browser still delivers the keypress to
  // xterm's hidden input textarea, which used to leak a literal "v"/"c"
  // character into the terminal right before the actual paste/copy ran.
  term.attachCustomKeyEventHandler((event) => {
    if (event.type !== "keydown") return true;
    const onlyShift = event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey;
    if (onlyShift && event.key.toLowerCase() === "c") {
      event.preventDefault();
      const selection = term.getSelection();
      if (selection) {
        void navigator.clipboard.writeText(selection);
      }
      return false;
    }
    if (onlyShift && event.key.toLowerCase() === "v") {
      event.preventDefault();
      navigator.clipboard.readText().then((text) => {
        if (text) void invoke("write_pty", { serverId, data: text });
      });
      return false;
    }
    return true;
  });

  term.onData((data) => {
    void invoke("write_pty", { serverId, data }).catch((e) => console.error("write_pty failed", e));
  });

  term.onResize(({ cols, rows }) => {
    void invoke("resize_pty", { serverId, cols, rows }).catch((e) => console.error("resize_pty failed", e));
  });

  const unlistenOutput = await listen<string>(`pty-output:${serverId}`, (event) => {
    term.write(b64ToBytes(event.payload));
  });
  const unlistenClosed = await listen(`pty-closed:${serverId}`, () => {
    term.write("\r\n\x1b[31m[соединение закрыто]\x1b[0m\r\n");
  });

  sessions.set(serverId, { serverId, term, fit, container, unlistenOutput, unlistenClosed });

  await invoke("open_terminal", { serverId, cols: term.cols, rows: term.rows });
  term.focus();
}

export function showTerminal(serverId: string): void {
  for (const s of sessions.values()) {
    s.container.style.display = s.serverId === serverId ? "block" : "none";
  }
  const s = sessions.get(serverId);
  if (s) {
    s.fit.fit();
    s.term.focus();
  }
}

export function closeTerminal(serverId: string): void {
  const s = sessions.get(serverId);
  if (!s) return;
  s.unlistenOutput();
  s.unlistenClosed();
  s.term.dispose();
  s.container.remove();
  sessions.delete(serverId);
  void invoke("disconnect_server", { serverId });
}

export function hasTerminal(serverId: string): boolean {
  return sessions.has(serverId);
}

export function refit(serverId: string): void {
  sessions.get(serverId)?.fit.fit();
}
