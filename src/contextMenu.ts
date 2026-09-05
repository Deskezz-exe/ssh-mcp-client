export interface ContextMenuItem {
  label: string;
  onClick: () => void;
  danger?: boolean;
}

let menuEl: HTMLDivElement | null = null;

function closeMenu(): void {
  menuEl?.remove();
  menuEl = null;
  document.removeEventListener("click", closeMenu);
  document.removeEventListener("contextmenu", closeOnOutsideContext);
  document.removeEventListener("keydown", onKeydown);
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape") closeMenu();
}

function closeOnOutsideContext(e: MouseEvent): void {
  if (menuEl && !menuEl.contains(e.target as Node)) closeMenu();
}

export function showContextMenu(x: number, y: number, items: ContextMenuItem[]): void {
  closeMenu();

  const menu = document.createElement("div");
  menu.className = "context-menu";
  menu.style.left = `${x}px`;
  menu.style.top = `${y}px`;

  for (const item of items) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "context-menu-item" + (item.danger ? " danger" : "");
    btn.textContent = item.label;
    btn.addEventListener("click", () => {
      closeMenu();
      item.onClick();
    });
    menu.appendChild(btn);
  }

  document.body.appendChild(menu);
  menuEl = menu;

  // Nudge back on-screen if it would overflow the right/bottom edge.
  const rect = menu.getBoundingClientRect();
  if (rect.right > window.innerWidth) {
    menu.style.left = `${Math.max(0, window.innerWidth - rect.width - 8)}px`;
  }
  if (rect.bottom > window.innerHeight) {
    menu.style.top = `${Math.max(0, window.innerHeight - rect.height - 8)}px`;
  }

  // Deferred so the contextmenu/click event that opened the menu doesn't
  // immediately close it via these same-type listeners.
  setTimeout(() => {
    document.addEventListener("click", closeMenu);
    document.addEventListener("contextmenu", closeOnOutsideContext);
    document.addEventListener("keydown", onKeydown);
  }, 0);
}
