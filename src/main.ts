import { api } from "./lib/api";
import type { ConnectionProfile, FileEntry } from "./lib/types";
import { renderConnectionBar } from "./components/ConnectionBar";
import { renderToolbar } from "./components/Toolbar";
import { renderFileTable } from "./components/FileTable";
import { renderProgress } from "./components/Progress";
import { showPreview } from "./components/PreviewModal";
import { formatSize } from "./lib/format";
import { storage } from "./lib/storage";

const app = document.getElementById("app")!;
app.innerHTML = `
  <div class="layout">
    <div class="topbar">
      <h1>🗂️ WebDAV 客户端</h1>
      <span id="active-badge" class="badge" style="display:none"></span>
      <div class="spacer"></div>
      <span id="status" class="status">未连接</span>
    </div>
    <div id="connection"></div>
    <div id="toolbar" class="toolbar" style="display:none"></div>
    <div class="main" id="main"><div class="empty">选择或新增连接</div></div>
    <div id="progress"></div>
    <div class="footer"><span id="footer-left"></span><span id="footer-right"></span></div>
  </div>`;

let activeConnectionId: string | null = storage.getActiveConnectionId();
let activeProfile: ConnectionProfile | null = null;
let currentPath = "";
let entries: FileEntry[] = [];
let query = "";

const statusEl = document.getElementById("status")!;
const toolbarEl = document.getElementById("toolbar")!;
const mainEl = document.getElementById("main")!;
const badgeEl = document.getElementById("active-badge")!;
const progress = renderProgress(document.getElementById("progress")!);

function setActiveUi() {
  if (activeConnectionId && activeProfile) {
    badgeEl.textContent = activeProfile.name || activeProfile.url;
    badgeEl.style.display = "";
    toolbarEl.style.display = "flex";
  } else {
    badgeEl.style.display = "none";
    toolbarEl.style.display = "none";
  }
}

function setStatus(text: string, cls = "") { statusEl.textContent = text; statusEl.className = "status " + cls; }
function toast(msg: string, isErr = false) {
  const t = document.createElement("div");
  t.className = "toast" + (isErr ? " err" : "");
  t.textContent = msg;
  document.body.appendChild(t);
  setTimeout(() => t.remove(), 3200);
}

function connId() { return activeConnectionId ?? undefined; }

function clearSession() {
  activeConnectionId = null;
  activeProfile = null;
  storage.setActiveConnectionId(null);
  currentPath = "";
  query = "";
  entries = [];
  setStatus("未连接");
  mainEl.innerHTML = `<div class="empty">已断开。请选择或新增连接后点击连接。</div>`;
  document.getElementById("footer-left")!.textContent = "";
  document.getElementById("footer-right")!.textContent = "";
  setActiveUi();
}

async function refresh() {
  if (!activeConnectionId) return;
  try {
    entries = await api.listDir(currentPath, connId());
    drawToolbar();
    renderFileTable(mainEl, entries, query, tableEvents);
    const total = entries.reduce((s, e) => s + e.size, 0);
    document.getElementById("footer-left")!.textContent = `${entries.length} 项 · ${formatSize(total)}`;
    document.getElementById("footer-right")!.textContent = activeProfile?.url ?? "";
  } catch (e) {
    mainEl.innerHTML = `<div class="empty">加载失败：${e}</div>`;
    toast("加载失败：" + e, true);
  }
}

function drawToolbar() {
  renderToolbar(toolbarEl, currentPath, {
    onNavigate: (p) => { currentPath = p; refresh(); },
    onCreateFolder: async () => {
      const name = prompt("文件夹名称");
      if (!name) return;
      try { await api.createFolder((currentPath ? currentPath + "/" : "") + name, connId()); refresh(); }
      catch (e) { toast("创建失败：" + e, true); }
    },
    onUpload: uploadFiles,
    onRefresh: refresh,
    onSearch: (q) => { query = q; renderFileTable(mainEl, entries, query, tableEvents); },
  });
}

const tableEvents = {
  onOpen: (e: FileEntry) => { if (e.isDir) { currentPath = e.path; refresh(); } },
  onDownload: async (e: FileEntry) => {
    try {
      const saved = await api.downloadFile(e.path, connId());
      toast("已保存到 " + saved);
    } catch (err) { toast("下载失败：" + err, true); }
  },
  onPreview: async (e: FileEntry) => {
    try { await showPreview(e.path, e.name, connId()); }
    catch (err) { toast("预览失败：" + err, true); }
  },
  onRename: async (e: FileEntry) => {
    const name = prompt("新名称", e.name);
    if (!name || name === e.name) return;
    try { await api.renameEntry(e.path, name, connId()); refresh(); }
    catch (err) { toast("重命名失败：" + err, true); }
  },
  onDelete: async (e: FileEntry) => {
    if (!confirm(`确定删除「${e.name}」？`)) return;
    try { await api.deleteEntry(e.path, connId()); refresh(); }
    catch (err) { toast("删除失败：" + err, true); }
  },
};

async function uploadFiles() {
  if (!activeConnectionId) return toast("请先选择连接", true);
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({ multiple: true, directory: false });
  if (!selected) return;
  const files = Array.isArray(selected) ? selected : [selected];
  if (!files.length) return;
  // currentPath is relative to the DAV root URL (e.g. connection http://host/dav + currentPath "/webdav"
  // -> PUT http://host/dav/webdav/{filename}).
  for (const f of files) {
    const transferId = crypto.randomUUID();
    try {
      await api.uploadLocalFile(f, transferId, connId(), currentPath || undefined);
      toast("上传完成：" + f.split(/[/\\]/).pop());
    } catch (e) { toast("上传失败：" + e, true); }
    finally { progress.done(transferId); }
  }
  refresh();
}

api.onProgress((e) => progress.update(e)).catch(() => {});

const connBar = renderConnectionBar(document.getElementById("connection")!, {
  onConnect: (id, profile) => {
    activeConnectionId = id;
    activeProfile = profile;
    storage.setActiveConnectionId(id);
    currentPath = "";
    query = "";
    setStatus("已连接 " + (profile.name || profile.url), "ok");
    setActiveUi();
    refresh();
  },
  onDisconnect: () => {
    clearSession();
  },
  onAdd: () => {},
}, () => activeConnectionId);

// Auto-restore last used connection
(async () => {
  const connections = await api.listConnections().catch(() => []);
  if (!connections.length) return;
  const target = activeConnectionId && connections.some((c) => c.id === activeConnectionId)
    ? activeConnectionId
    : connections.slice().sort((a, b) => (b.lastUsed ?? 0) - (a.lastUsed ?? 0))[0].id;
  const profile = await api.getConnection(target);
  if (!profile) return;
  try {
    await api.connect(target);
    activeConnectionId = target;
    activeProfile = profile;
    setStatus("已连接 " + (profile.name || profile.url), "ok");
    setActiveUi();
    await refresh();
    connBar.refresh();
  } catch {}
})();
