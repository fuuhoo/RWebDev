export interface ToolbarEvents {
  onNavigate: (path: string) => void;
  onCreateFolder: () => void;
  onUpload: () => void;
  onRefresh: () => void;
  onSearch: (q: string) => void;
}

export function renderToolbar(root: HTMLElement, currentPath: string, events: ToolbarEvents) {
  const parts = currentPath.split("/").filter(Boolean);
  const crumbs = parts.map((p, i) => `<button data-i="${i}">${escapeHtml(p)}</button>`).join("<span>/</span>");
  root.innerHTML = `
    <div class="breadcrumb"><button data-i="-1">根目录</button>${crumbs ? `<span>/</span>${crumbs}` : ""}</div>
    <input class="search" id="tb-search" placeholder="搜索当前目录…" />
    <button id="tb-refresh" class="ghost">刷新</button>
    <button id="tb-mkdir" class="ghost">新建文件夹</button>
    <button id="tb-upload">上传文件</button>`;
  root.querySelectorAll(".breadcrumb button").forEach((b) =>
    b.addEventListener("click", () => {
      const i = Number((b as HTMLElement).dataset.i);
      events.onNavigate(i === -1 ? "" : "/" + parts.slice(0, i + 1).join("/"));
    })
  );
  root.querySelector("#tb-refresh")!.addEventListener("click", events.onRefresh);
  root.querySelector("#tb-mkdir")!.addEventListener("click", events.onCreateFolder);
  root.querySelector("#tb-upload")!.addEventListener("click", events.onUpload);
  root.querySelector("#tb-search")!.addEventListener("input", (e) =>
    events.onSearch((e.target as HTMLInputElement).value)
  );
}

function escapeHtml(s: string) {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
}
