import { invoke } from "@tauri-apps/api/core";
import { openTerminal, showTerminal, closeTerminal, hasTerminal, refit } from "./terminal";
import { colorForServer } from "./cardColors";
import { showContextMenu } from "./contextMenu";

interface ServerSummary {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  connected: boolean;
  favorite: boolean;
}

type TabKind = "term" | "sftp";

let activeTab = "home";
let selectedServerId: string | null = null;

function tabKey(kind: TabKind, serverId: string): string {
  return `${kind}-${serverId}`;
}

function escapeHtml(s: string): string {
  const div = document.createElement("div");
  div.textContent = s;
  return div.innerHTML;
}

async function loadServers(): Promise<ServerSummary[]> {
  return invoke<ServerSummary[]>("list_servers");
}

function renderServerGrid(servers: ServerSummary[]): void {
  const grid = document.querySelector<HTMLDivElement>("#server-grid")!;
  grid.innerHTML = "";

  if (servers.length === 0) {
    const empty = document.createElement("div");
    empty.id = "grid-empty";
    empty.textContent = "Серверов пока нет — добавь первый.";
    grid.appendChild(empty);
    return;
  }

  for (const s of servers) {
    const card = document.createElement("div");
    card.className = "server-card" + (s.id === selectedServerId ? " selected" : "");
    card.style.setProperty("--card-accent", colorForServer(s.id));
    card.innerHTML = `
      <div class="server-card-top">
        <span class="dot ${s.connected ? "on" : ""}"></span>
        <span class="server-card-name">${escapeHtml(s.name)}</span>
        <button type="button" class="star ${s.favorite ? "filled" : ""}" title="${
          s.favorite ? "Убрать из избранного" : "Добавить в избранное"
        }">${s.favorite ? "★" : "☆"}</button>
      </div>
      <div class="server-card-target">${escapeHtml(s.username)}@${escapeHtml(s.host)}:${s.port}</div>
      <div class="server-card-hint">Двойной клик — подключиться, ПКМ — меню</div>
    `;

    card.querySelector(".star")!.addEventListener("click", (e) => {
      e.stopPropagation();
      void toggleFavorite(s);
    });

    card.addEventListener("click", () => {
      selectedServerId = s.id;
      void refresh();
    });
    card.addEventListener("dblclick", () => void connectAndOpen(s));
    card.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      selectedServerId = s.id;
      showContextMenu(e.clientX, e.clientY, [
        { label: "Подключиться", onClick: () => void connectAndOpen(s) },
        { label: "Подключиться через ФМ", onClick: () => void openSftpTab(s) },
        {
          label: s.favorite ? "Убрать из избранного" : "Добавить в избранное",
          onClick: () => void toggleFavorite(s),
        },
        { label: "Удалить", danger: true, onClick: () => void deleteServer(s) },
      ]);
    });

    const delBtn = document.createElement("button");
    delBtn.textContent = "×";
    delBtn.className = "delete-btn";
    delBtn.title = "Удалить сервер";
    delBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      void deleteServer(s);
    });
    card.appendChild(delBtn);

    grid.appendChild(card);
  }
}

async function toggleFavorite(s: ServerSummary): Promise<void> {
  await invoke("set_favorite", { serverId: s.id, favorite: !s.favorite });
  await refresh();
}

async function deleteServer(s: ServerSummary): Promise<void> {
  closeTerminal(s.id);
  document.getElementById(`sftp-${s.id}`)?.remove();
  document.getElementById(`tab-${tabKey("term", s.id)}`)?.remove();
  document.getElementById(`tab-${tabKey("sftp", s.id)}`)?.remove();
  if (activeTab === tabKey("term", s.id) || activeTab === tabKey("sftp", s.id)) {
    setActiveTab("home");
  }
  await invoke("delete_profile", { serverId: s.id });
  await refresh();
}

function ensureTerminalContainer(serverId: string): HTMLElement {
  const terminalsEl = document.querySelector<HTMLDivElement>("#terminals")!;
  let container = document.getElementById(tabKey("term", serverId));
  if (!container) {
    container = document.createElement("div");
    container.id = tabKey("term", serverId);
    container.className = "terminal-container";
    terminalsEl.appendChild(container);
  }
  return container;
}

function ensureSftpContainer(serverId: string): HTMLElement {
  const terminalsEl = document.querySelector<HTMLDivElement>("#terminals")!;
  let container = document.getElementById(tabKey("sftp", serverId));
  if (!container) {
    container = document.createElement("div");
    container.id = tabKey("sftp", serverId);
    container.className = "sftp-container";
    container.innerHTML = `<div class="sftp-placeholder">Файловый менеджер появится в следующем обновлении.</div>`;
    terminalsEl.appendChild(container);
  }
  return container;
}

function ensureTab(kind: TabKind, server: ServerSummary): void {
  const key = tabKey(kind, server.id);
  const sessionTabs = document.querySelector<HTMLDivElement>("#session-tabs")!;
  if (document.getElementById(`tab-${key}`)) return;

  const tab = document.createElement("div");
  tab.id = `tab-${key}`;
  tab.className = "tab";
  const label = document.createElement("span");
  label.textContent = (kind === "term" ? "🖥 " : "📁 ") + server.name;
  tab.appendChild(label);

  tab.addEventListener("click", () => setActiveTab(key));

  const closeBtn = document.createElement("span");
  closeBtn.textContent = " ×";
  closeBtn.className = "tab-close";
  closeBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    closeTabByKind(kind, server.id);
  });
  tab.appendChild(closeBtn);

  sessionTabs.appendChild(tab);
}

function closeTabByKind(kind: TabKind, serverId: string): void {
  const key = tabKey(kind, serverId);
  if (kind === "term") {
    closeTerminal(serverId);
  } else {
    document.getElementById(key)?.remove();
  }
  document.getElementById(`tab-${key}`)?.remove();
  if (activeTab === key) setActiveTab("home");
  void refresh();
}

function setActiveTab(key: string): void {
  activeTab = key;

  document.getElementById("tab-home")!.classList.toggle("active", key === "home");
  document.querySelectorAll<HTMLElement>("#session-tabs .tab").forEach((el) => {
    el.classList.toggle("active", el.id === `tab-${key}`);
  });

  const dashboard = document.querySelector<HTMLElement>("#dashboard")!;
  dashboard.style.display = key === "home" ? "flex" : "none";

  document.querySelectorAll<HTMLElement>(".terminal-container, .sftp-container").forEach((el) => {
    el.style.display = "none";
  });

  if (key.startsWith("term-")) {
    showTerminal(key.slice("term-".length));
  } else if (key.startsWith("sftp-")) {
    const el = document.getElementById(key);
    if (el) el.style.display = "block";
  }
}

async function connectAndOpen(server: ServerSummary): Promise<void> {
  ensureTab("term", server);
  const container = ensureTerminalContainer(server.id);
  setActiveTab(tabKey("term", server.id));

  if (!hasTerminal(server.id)) {
    try {
      await openTerminal(server.id, container);
      // The container was still hidden (display: none) when openTerminal()
      // called term.open()/fit.fit(), since the session didn't exist yet
      // for showTerminal() to find above. Show it again now that it does,
      // so xterm gets a real size to fit into.
      showTerminal(server.id);
    } catch (e) {
      container.textContent = `Не удалось подключиться: ${String(e)}`;
    }
  }
  await refresh();
}

async function openSftpTab(server: ServerSummary): Promise<void> {
  ensureTab("sftp", server);
  ensureSftpContainer(server.id);
  setActiveTab(tabKey("sftp", server.id));
  await refresh();
}

async function refresh(): Promise<void> {
  const servers = await loadServers();
  renderServerGrid(servers);
}

function openAddServerForm(): void {
  document.querySelector<HTMLDivElement>("#add-server-overlay")!.style.display = "flex";
  document.querySelector<HTMLInputElement>("#f-name")!.focus();
}

function closeAddServerForm(): void {
  document.querySelector<HTMLDivElement>("#add-server-overlay")!.style.display = "none";
  document.querySelector<HTMLFormElement>("#add-server-form")!.reset();
  document.querySelector<HTMLInputElement>("#f-port")!.value = "22";
}

window.addEventListener("DOMContentLoaded", () => {
  document.getElementById("tab-home")!.addEventListener("click", () => setActiveTab("home"));

  document.getElementById("add-server-btn")!.addEventListener("click", openAddServerForm);
  document.getElementById("add-server-cancel")!.addEventListener("click", closeAddServerForm);

  const form = document.querySelector<HTMLFormElement>("#add-server-form")!;
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    void (async () => {
      const name = document.querySelector<HTMLInputElement>("#f-name")!.value;
      const host = document.querySelector<HTMLInputElement>("#f-host")!.value;
      const port = parseInt(document.querySelector<HTMLInputElement>("#f-port")!.value, 10);
      const username = document.querySelector<HTMLInputElement>("#f-username")!.value;
      const password = document.querySelector<HTMLInputElement>("#f-password")!.value;
      await invoke("save_profile", { name, host, port, username, password });
      closeAddServerForm();
      await refresh();
    })();
  });

  window.addEventListener("resize", () => {
    if (activeTab.startsWith("term-")) refit(activeTab.slice("term-".length));
  });

  setActiveTab("home");
  void refresh();
});
