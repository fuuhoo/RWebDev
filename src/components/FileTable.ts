import { formatDate, formatSize, iconFor } from "../lib/format";
import type { FileEntry } from "../lib/types";

export interface TableEvents {
  onOpen: (e: FileEntry) => void;
  onDownload: (e: FileEntry) => void;
  onPreview: (e: FileEntry) => void;
  onRename: (e: FileEntry) => void;
  onDelete: (e: FileEntry) => void;
}

let sortKey: "name" | "size" | "modified" = "name";
let sortAsc = true;

export function renderFileTable(root: HTMLElement, entries: FileEntry[], query: string, events: TableEvents) {
  const filtered = entries.filter((e) => e.name.toLowerCase().includes(query.toLowerCase()));
  const sorted = [...filtered].sort((a, b) => {
    if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
    let c = 0;
    if (sortKey === "name") c = a.name.localeCompare(b.name, "zh-CN");
    if (sortKey === "size") c = a.size - b.size;
    if (sortKey === "modified") c = (a.modified ?? "").localeCompare(b.modified ?? "");
    return sortAsc ? c : -c;
  });

  if (sorted.length === 0) {
    root.innerHTML = `<div class="empty">空文件夹</div>`;
    return;
  }
  const th = (k: "name" | "size" | "modified", label: string) =>
    `<th data-k="${k}">${label}${sortKey === k ? (sortAsc ? " ↑" : " ↓") : ""}</th>`;
  root.innerHTML = `
    <table>
      <thead><tr>${th("name", "名称")}${th("size", "大小")}${th("modified", "修改时间")}<th style="text-align:right">操作</th></tr></thead>
      <tbody>
        ${sorted.map((e) => `
          <tr data-path="${e.path}">
            <td><span class="name-cell"><span>${iconFor(e.name, e.isDir)}</span>${escapeHtml(e.name)}</span></td>
            <td>${e.isDir ? "—" : formatSize(e.size)}</td>
            <td>${formatDate(e.modified)}</td>
            <td><div class="actions">
              ${e.isDir ? "" : `<button class="ghost small" data-act="preview">预览</button>
                                <button class="ghost small" data-act="download">下载</button>`}
              <button class="ghost small" data-act="rename">重命名</button>
              <button class="ghost small danger" data-act="delete">删除</button>
            </div></td>
          </tr>`).join("")}
      </tbody>
    </table>`;
  root.querySelectorAll("th[data-k]").forEach((el) => {
    el.addEventListener("click", () => {
      const k = (el as HTMLElement).dataset.k as typeof sortKey;
      if (k === sortKey) sortAsc = !sortAsc; else { sortKey = k; sortAsc = true; }
      renderFileTable(root, entries, query, events);
    });
  });
  root.querySelectorAll("tbody tr[data-path]").forEach((tr) => {
    const entry = sorted.find((e) => e.path === (tr as HTMLElement).dataset.path)!;
    tr.addEventListener("dblclick", () => events.onOpen(entry));
    tr.querySelectorAll("button[data-act]").forEach((btn) => {
      btn.addEventListener("click", (ev) => {
        ev.stopPropagation();
        const act = (btn as HTMLElement).dataset.act;
        if (act === "download") events.onDownload(entry);
        if (act === "preview") events.onPreview(entry);
        if (act === "rename") events.onRename(entry);
        if (act === "delete") events.onDelete(entry);
      });
    });
  });
}

function escapeHtml(s: string) {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
}
