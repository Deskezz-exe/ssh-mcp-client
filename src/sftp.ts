import { invoke } from "@tauri-apps/api/core";

interface RemoteEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

interface LocalEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

interface LocalListing {
  parent: string | null;
  entries: LocalEntry[];
}

type Side = "local" | "remote";

interface PaneState {
  path: string;
  selected: string | null;
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
      <div class="sftp-transfer-controls">
        <button type="button" class="sftp-arrow-right" title="Загрузить на сервер" disabled>→</button>
        <button type="button" class="sftp-arrow-left" title="Скачать на компьютер" disabled>←</button>
      </div>
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
    local: { path: "", selected: null },
    remote: { path: ".", selected: null },
  };
  views.set(serverId, state);

  wirePane(state, "local");
  wirePane(state, "remote");

  container.querySelector(".sftp-arrow-right")!.addEventListener("click", () => void transfer(state, "up"));
  container.querySelector(".sftp-arrow-left")!.addEventListener("click", () => void transfer(state, "down"));

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
      const entries = await invoke<RemoteEntry[]>("list_remote_directory", {
        serverId: state.serverId,
        path: state.remote.path,
      });
      pathInput.value = state.remote.path;
      renderList(state, side, entries, paneEl);
    }
  } catch (e) {
    listEl.innerHTML = `<div class="sftp-error"></div>`;
    listEl.querySelector(".sftp-error")!.textContent = `Не удалось открыть "${state[side].path}": ${String(e)}`;
  }
}

function renderList(state: SftpViewState, side: Side, entries: RemoteEntry[] | LocalEntry[], paneEl: HTMLElement): void {
  const listEl = paneEl.querySelector<HTMLElement>(".sftp-list")!;
  listEl.innerHTML = "";
  state[side].selected = null;
  updateArrowButtons(state);

  for (const entry of entries) {
    const row = document.createElement("div");
    row.className = "sftp-row" + (entry.is_dir ? " sftp-dir" : "");

    const name = document.createElement("span");
    name.className = "sftp-name";
    name.textContent = (entry.is_dir ? "📁 " : "📄 ") + entry.name;
    row.appendChild(name);

    if (!entry.is_dir) {
      const size = document.createElement("span");
      size.className = "sftp-size";
      size.textContent = formatSize(entry.size);
      row.appendChild(size);
    }

    row.addEventListener("click", () => {
      if (entry.is_dir) {
        state[side].path = entry.path;
        void loadPane(state, side);
        return;
      }
      paneEl.querySelectorAll(".sftp-row.selected").forEach((el) => el.classList.remove("selected"));
      row.classList.add("selected");
      state[side].selected = entry.path;
      updateArrowButtons(state);
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

function updateArrowButtons(state: SftpViewState): void {
  const rightArrow = state.container.querySelector<HTMLButtonElement>(".sftp-arrow-right")!;
  const leftArrow = state.container.querySelector<HTMLButtonElement>(".sftp-arrow-left")!;
  rightArrow.disabled = !state.local.selected;
  leftArrow.disabled = !state.remote.selected;
}

async function transfer(state: SftpViewState, direction: "up" | "down"): Promise<void> {
  if (direction === "up") {
    const localPath = state.local.selected;
    if (!localPath) return;
    const filename = localPath.split(/[\\/]/).pop()!;
    const remotePath = joinRemote(state.remote.path, filename);
    try {
      await invoke("upload_to_server", { serverId: state.serverId, localPath, remotePath });
      await loadPane(state, "remote");
    } catch (e) {
      alert(`Не удалось загрузить файл: ${String(e)}`);
    }
  } else {
    const remotePath = state.remote.selected;
    if (!remotePath) return;
    const filename = remotePath.split("/").pop()!;
    const localPath = joinLocal(state.local.path, filename);
    try {
      await invoke("download_from_server", { serverId: state.serverId, remotePath, localPath });
      await loadPane(state, "local");
    } catch (e) {
      alert(`Не удалось скачать файл: ${String(e)}`);
    }
  }
}
