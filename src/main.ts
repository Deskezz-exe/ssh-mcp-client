import { invoke } from "@tauri-apps/api/core";
import { openTerminal, showTerminal, closeTerminal, hasTerminal, refit } from "./terminal";
import { colorForServer } from "./cardColors";

interface ServerSummary {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  connected: boolean;
}

let activeTab = "home";
let selectedServerId: string | null = null;

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
      </div>
      <div class="server-card-target">${escapeHtml(s.username)}@${escapeHtml(s.host)}:${s.port}</div>
      <div class="server-card-hint">Двойной клик — подключиться</div>
    `;

    card.addEventListener("click", () => {
      selectedServerId = s.id;
      void refresh();
    });
    card.addEventListener("dblclick", () => void connectAndOpen(s));

    const delBtn = document.createElement("button");
    delBtn.textContent = "×";
    delBtn.className = "delete-btn";
    delBtn.title = "Удалить сервер";
    delBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      void (async () => {
        closeTerminal(s.id);
        removeSessionTab(s.id);
        await invoke("delete_profile", { serverId: s.id });
        await refresh();
      })();
    });
    card.appendChild(delBtn);

    grid.appendChild(card);
  }
}

function ensureTerminalContainer(serverId: string): HTMLElement {
  const terminalsEl = document.querySelector<HTMLDivElement>("#terminals")!;
  let container = document.getElementById(`term-${serverId}`);
  if (!container) {
    container = document.createElement("div");
    container.id = `term-${serverId}`;
    container.className = "terminal-container";
    terminalsEl.appendChild(container);
  }
  return container;
}

function ensureSessionTab(server: ServerSummary): void {
  const sessionTabs = document.querySelector<HTMLDivElement>("#session-tabs")!;
  let tab = document.getElementById(`tab-${server.id}`);
  if (tab) return;

  tab = document.createElement("div");
  tab.id = `tab-${server.id}`;
  tab.className = "tab";
  const label = document.createElement("span");
  label.textContent = server.name;
  tab.appendChild(label);

  tab.addEventListener("click", () => setActiveTab(server.id));

  const closeBtn = document.createElement("span");
  closeBtn.textContent = " ×";
  closeBtn.className = "tab-close";
  closeBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    closeTerminal(server.id);
    removeSessionTab(server.id);
    if (activeTab === server.id) setActiveTab("home");
    void refresh();
  });
  tab.appendChild(closeBtn);

  sessionTabs.appendChild(tab);
}

function removeSessionTab(serverId: string): void {
  document.getElementById(`tab-${serverId}`)?.remove();
}

function setActiveTab(tab: string): void {
  activeTab = tab;

  document.getElementById("tab-home")!.classList.toggle("active", tab === "home");
  document.querySelectorAll<HTMLElement>("#session-tabs .tab").forEach((el) => {
    el.classList.toggle("active", el.id === `tab-${tab}`);
  });

  const dashboard = document.querySelector<HTMLElement>("#dashboard")!;
  dashboard.style.display = tab === "home" ? "flex" : "none";

  if (tab === "home") {
    document.querySelectorAll<HTMLElement>(".terminal-container").forEach((el) => {
      el.style.display = "none";
    });
  } else {
    showTerminal(tab);
  }
}

async function connectAndOpen(server: ServerSummary): Promise<void> {
  ensureSessionTab(server);
  const container = ensureTerminalContainer(server.id);
  setActiveTab(server.id);

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
    if (activeTab !== "home") refit(activeTab);
  });

  setActiveTab("home");
  void refresh();
});
