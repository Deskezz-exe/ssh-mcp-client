import { invoke } from "@tauri-apps/api/core";
import { showContextMenu, type ContextMenuItem } from "./contextMenu";

interface RemoteEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: number | null;
}

interface RemoteListing {
  current: string;
  entries: RemoteEntry[];
}

interface LocalEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: number | null;
}

interface LocalListing {
  parent: string | null;
  entries: LocalEntry[];
}

type Side = "local" | "remote";

interface PaneState {
  path: string;
}

interface SftpViewState {
  serverId: string;
  container: HTMLElement;
  local: PaneState;
  remote: PaneState;
}

const views = new Map<string, SftpViewState>();

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let val = bytes / 1024;
  let i = 0;
  while (val >= 1024 && i < units.length - 1) {
    val /= 1024;
    i++;
  }
  return `${val.toFixed(1)} ${units[i]}`;
}

function formatDate(unixSeconds: number | null): string {
  if (unixSeconds == null) return "";
  const d = new Date(unixSeconds * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getDate())}.${pad(d.getMonth() + 1)}.${d.getFullYear()} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function joinRemote(dir: string, name: string): string {
  return dir.endsWith("/") ? `${dir}${name}` : `${dir}/${name}`;
}

function joinLocal(dir: string, name: string): string {
  const sep = dir.includes("\\") && !dir.includes("/") ? "\\" : "/";
  return dir.endsWith("\\") || dir.endsWith("/") ? `${dir}${name}` : `${dir}${sep}${name}`;
}

export async function openSftpView(serverId: string, container: HTMLElement): Promise<void> {
  if (views.has(serverId)) return;

  container.innerHTML = `
    <div class="sftp-panes">
      <div class="sftp-pane" data-side="local">
        <div class="sftp-pane-header">Этот компьютер</div>
        <div class="sftp-toolbar">
          <button type="button" class="sftp-up" title="Вверх">⬆</button>
          <input type="text" class="sftp-path" spellcheck="false" />
          <button type="button" class="sftp-go">Перейти</button>
        </div>
        <div class="sftp-list"></div>
      </div>
      <div class="sftp-divider"></div>
      <div class="sftp-pane" data-side="remote">
        <div class="sftp-pane-header">Сервер</div>
        <div class="sftp-toolbar">
          <button type="button" class="sftp-up" title="Вверх">⬆</button>
          <input type="text" class="sftp-path" spellcheck="false" />
          <button type="button" class="sftp-go">Перейти</button>
        </div>
        <div class="sftp-list"></div>
      </div>
    </div>
  `;

  const state: SftpViewState = {
    serverId,
    container,
    local: { path: "" },
    remote: { path: "." },
  };
  views.set(serverId, state);

  wirePane(state, "local");
  wirePane(state, "remote");

  try {
    state.local.path = await invoke<string>("get_home_dir");
  } catch (e) {
    console.error("get_home_dir failed", e);
  }

  await loadPane(state, "local");
  await loadPane(state, "remote");
}

export function closeSftpView(serverId: string): void {
  views.delete(serverId);
}

function wirePane(state: SftpViewState, side: Side): void {
  const paneEl = state.container.querySelector<HTMLElement>(`[data-side="${side}"]`)!;
  const pathInput = paneEl.querySelector<HTMLInputElement>(".sftp-path")!;

  paneEl.querySelector(".sftp-up")!.addEventListener("click", () => void navigateUp(state, side));
  paneEl.querySelector(".sftp-go")!.addEventListener("click", () => {
    state[side].path = pathInput.value;
    void loadPane(state, side);
  });
  pathInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      state[side].path = pathInput.value;
      void loadPane(state, side);
    }
  });

  paneEl.querySelector(".sftp-list")!.addEventListener("contextmenu", (e) => {
    const target = e.target as HTMLElement;
    if (target.closest(".sftp-row")) return; // row has its own handler
    e.preventDefault();
    showContextMenu((e as MouseEvent).clientX, (e as MouseEvent).clientY, [
      { label: "Обновить", onClick: () => void loadPane(state, side) },
    ]);
  });
}

async function navigateUp(state: SftpViewState, side: Side): Promise<void> {
  if (side === "remote") {
    const trimmed = state.remote.path.replace(/\/+$/, "");
    const parent = trimmed.split("/").slice(0, -1).join("/");
    state.remote.path = parent || "/";
    await loadPane(state, "remote");
    return;
  }

  try {
    const listing = await invoke<LocalListing>("list_local_directory", { path: state.local.path });
    if (listing.parent) {
      state.local.path = listing.parent;
      await loadPane(state, "local");
    }
  } catch (e) {
    console.error("list_local_directory (parent) failed", e);
  }
}

async function loadPane(state: SftpViewState, side: Side): Promise<void> {
  const paneEl = state.container.querySelector<HTMLElement>(`[data-side="${side}"]`)!;
  const listEl = paneEl.querySelector<HTMLElement>(".sftp-list")!;
  const pathInput = paneEl.querySelector<HTMLInputElement>(".sftp-path")!;
  listEl.innerHTML = `<div class="sftp-loading">Загрузка…</div>`;

  try {
    if (side === "local") {
      const listing = await invoke<LocalListing>("list_local_directory", { path: state.local.path });
      pathInput.value = state.local.path;
      renderList(state, side, listing.entries, paneEl);
    } else {
      const listing = await invoke<RemoteListing>("list_remote_directory", {
        serverId: state.serverId,
        path: state.remote.path,
      });
      state.remote.path = listing.current;
      pathInput.value = listing.current;
      renderList(state, side, listing.entries, paneEl);
    }
  } catch (e) {
    listEl.innerHTML = `<div class="sftp-error"></div>`;
    listEl.querySelector(".sftp-error")!.textContent = `Не удалось открыть "${state[side].path}": ${String(e)}`;
  }
}

function renderList(state: SftpViewState, side: Side, entries: RemoteEntry[] | LocalEntry[], paneEl: HTMLElement): void {
  const listEl = paneEl.querySelector<HTMLElement>(".sftp-list")!;
  listEl.innerHTML = "";

  for (const entry of entries) {
    const row = document.createElement("div");
    row.className = "sftp-row" + (entry.is_dir ? " sftp-dir" : "");

    const name = document.createElement("span");
    name.className = "sftp-name";
    name.textContent = (entry.is_dir ? "📁 " : "📄 ") + entry.name;
    row.appendChild(name);

    if (!entry.is_dir) {
      const date = document.createElement("span");
      date.className = "sftp-date";
      date.textContent = formatDate(entry.modified);
      row.appendChild(date);

      const size = document.createElement("span");
      size.className = "sftp-size";
      size.textContent = formatSize(entry.size);
      row.appendChild(size);
    }

    row.addEventListener("click", () => {
      if (entry.is_dir) {
        state[side].path = entry.path;
        void loadPane(state, side);
      }
    });

    if (!entry.is_dir) {
      row.addEventListener("dblclick", () => void quickTransfer(state, side, entry));
    }

    row.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      e.stopPropagation();
      const items: ContextMenuItem[] = [];
      if (!entry.is_dir) {
        items.push({
          label: side === "local" ? "Загрузить на сервер" : "Скачать на компьютер",
          onClick: () => void quickTransfer(state, side, entry),
        });
      }
      items.push({ label: "Обновить", onClick: () => void loadPane(state, side) });
      if (side === "remote" && !entry.is_dir) {
        items.push({
          label: "Удалить с сервера",
          danger: true,
          onClick: () => void deleteRemote(state, entry as RemoteEntry),
        });
      }
      showContextMenu(e.clientX, e.clientY, items);
    });

    listEl.appendChild(row);
  }

  if (entries.length === 0) {
    const empty = document.createElement("div");
    empty.className = "sftp-empty";
    empty.textContent = "(пусто)";
    listEl.appendChild(empty);
  }
}

async function quickTransfer(state: SftpViewState, side: Side, entry: RemoteEntry | LocalEntry): Promise<void> {
  if (side === "local") {
    const remotePath = joinRemote(state.remote.path, entry.name);
    try {
      await invoke("upload_to_server", { serverId: state.serverId, localPath: entry.path, remotePath });
      await loadPane(state, "remote");
    } catch (e) {
      alert(`Не удалось загрузить файл: ${String(e)}`);
    }
  } else {
    const localPath = joinLocal(state.local.path, entry.name);
    try {
      await invoke("download_from_server", { serverId: state.serverId, remotePath: entry.path, localPath });
      await loadPane(state, "local");
    } catch (e) {
      alert(`Не удалось скачать файл: ${String(e)}`);
    }
  }
}

async function deleteRemote(state: SftpViewState, entry: RemoteEntry): Promise<void> {
  if (!confirm(`Удалить "${entry.name}" с сервера? Это необратимо.`)) return;
  try {
    await invoke("delete_remote_file", { serverId: state.serverId, path: entry.path });
    await loadPane(state, "remote");
  } catch (e) {
    alert(`Не удалось удалить файл: ${String(e)}`);
  }
}
