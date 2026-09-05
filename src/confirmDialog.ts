/** A styled yes/no modal, matching the app's other overlays — used instead
 * of the browser's native confirm() so it looks and feels consistent. */
export function confirmDialog(message: string, confirmLabel = "Заменить", cancelLabel = "Отмена"): Promise<boolean> {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "confirm-overlay";
    overlay.innerHTML = `
      <div class="confirm-box">
        <div class="confirm-message"></div>
        <div class="confirm-actions">
          <button type="button" class="confirm-cancel"></button>
          <button type="button" class="confirm-ok"></button>
        </div>
      </div>
    `;
    overlay.querySelector(".confirm-message")!.textContent = message;
    const cancelBtn = overlay.querySelector<HTMLButtonElement>(".confirm-cancel")!;
    const okBtn = overlay.querySelector<HTMLButtonElement>(".confirm-ok")!;
    cancelBtn.textContent = cancelLabel;
    okBtn.textContent = confirmLabel;

    function close(result: boolean): void {
      overlay.remove();
      document.removeEventListener("keydown", onKeydown);
      resolve(result);
    }
    function onKeydown(e: KeyboardEvent): void {
      if (e.key === "Escape") close(false);
    }

    cancelBtn.addEventListener("click", () => close(false));
    okBtn.addEventListener("click", () => close(true));
    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) close(false);
    });
    document.addEventListener("keydown", onKeydown);

    document.body.appendChild(overlay);
    okBtn.focus();
  });
}
