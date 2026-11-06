import type { ProgressEvent } from "../lib/types";

export function renderProgress(root: HTMLElement) {
  const active = new Map<string, ProgressEvent>();
  function draw() {
    root.innerHTML = [...active.values()]
      .map((p) => {
        const pct = p.total > 0 ? Math.min(100, (p.uploaded / p.total) * 100) : 0;
        return `<div class="progress"><div>${escapeHtml(p.fileName)} · ${(pct).toFixed(0)}%</div><div class="bar"><div style="width:${pct}%"></div></div></div>`;
      })
      .join("");
  }
  return {
    update(p: ProgressEvent) { active.set(p.transferId, p); draw(); },
    done(transferId: string) { active.delete(transferId); draw(); },
  };
}
function escapeHtml(s: string) {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
}
