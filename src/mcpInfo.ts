import { invoke } from "@tauri-apps/api/core";

async function init(): Promise<void> {
  let port: number;
  try {
    port = await invoke<number>("mcp_server_info");
  } catch (e) {
    console.error("mcp_server_info failed", e);
    return;
  }

  const url = `http://127.0.0.1:${port}/mcp`;
  const cmd = `claude mcp add --transport http ssh-mcp-client ${url}`;

  document.getElementById("mcp-info-port")!.textContent = url;
  const cmdEl = document.getElementById("mcp-info-cmd")!;
  cmdEl.textContent = cmd;

  document.getElementById("mcp-info-copy")?.addEventListener("click", () => {
    void navigator.clipboard.writeText(cmd);
  });
}

window.addEventListener("DOMContentLoaded", () => {
  void init();
});
