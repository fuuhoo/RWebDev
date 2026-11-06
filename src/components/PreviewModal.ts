import { api } from "../lib/api";
import { formatSize } from "../lib/format";
import type { PreviewData } from "../lib/types";

export async function showPreview(path: string, name: string, connectionId?: string) {
  const data: PreviewData = await api.previewFile(path, connectionId);
  const url = `data:${data.contentType};base64,${data.dataBase64}`;
  const body =
    data.kind === "image" ? `<img src="${url}" alt="" />` :
    data.kind === "video" ? `<video src="${url}" controls />` :
    data.kind === "audio" ? `<audio src="${url}" controls />` :
    data.kind === "pdf" ? `<embed src="${url}" type="application/pdf" width="100%" height="500" />` :
    data.kind === "text" ? `<pre>${escapeHtml(atob(data.dataBase64))}</pre>` :
    `<p>不支持预览的二进制文件（${formatSize(data.size)}）</p>`;

  const backdrop = document.createElement("div");
  backdrop.className = "modal-backdrop";
  backdrop.innerHTML = `
    <div class="modal" style="min-width:560px">
      <h3>${escapeHtml(name)}</h3>
      <div class="preview">${body}</div>
      <div class="row"><button class="ghost" id="pv-close">关闭</button></div>
    </div>`;
  document.body.appendChild(backdrop);
  const close = () => backdrop.remove();
  backdrop.addEventListener("click", (e) => { if (e.target === backdrop) close(); });
  backdrop.querySelector("#pv-close")!.addEventListener("click", close);
}

function escapeHtml(s: string) {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
}
