import { api } from "../lib/api";
import { formatRelativeTime } from "../lib/format";
import type { ConnectionProfile, ConnectionSummary } from "../lib/types";

export interface ConnectionBarEvents {
  onConnect: (id: string, profile: ConnectionProfile) => void;
  onDisconnect: () => void;
  onAdd: () => void;
}

/**
 * Renders:
 * - a "Manage connections" button (opens list pop-over/modal)
 * - a quick-select dropdown for the active connection
 * - a Disconnect button (only visible when a connection is active)
 * - an "Add connection" button (opens the create form)
 */

export function renderConnectionBar(
  root: HTMLElement,
  events: ConnectionBarEvents,
  getActiveId: () => string | null,
) {
  root.innerHTML = `
    <div class="connection">
      <button id="conn-manage" class="ghost">管理连接</button>
      <select id="conn-select" title="选择连接"></select>
      <button id="conn-disconnect" class="ghost" style="display:none">断开连接</button>
      <button id="conn-add" class="ghost">新增连接</button>
    </div>
    <div id="conn-form" class="connection connection-form" style="display:none">
      <input id="cfg-url" placeholder="服务器地址 https://dav.example.com/dav/" />
      <input id="cfg-user" placeholder="用户名" />
      <input id="cfg-pass" type="password" placeholder="密码" />
      <button id="cfg-connect">添加并连接</button>
    </div>`;

  const select = root.querySelector<HTMLSelectElement>("#conn-select")!;
  const form = root.querySelector<HTMLElement>("#conn-form")!;
  const urlInput = root.querySelector<HTMLInputElement>("#cfg-url")!;
  const userInput = root.querySelector<HTMLInputElement>("#cfg-user")!;
  const passInput = root.querySelector<HTMLInputElement>("#cfg-pass")!;
  const disconnectBtn = root.querySelector<HTMLButtonElement>("#conn-disconnect")!;

  function syncDisconnectVisibility() {
    const id = getActiveId();
    disconnectBtn.style.display = id ? "" : "none";
  }

  async function loadSelect() {
    const connections = await api.listConnections().catch(() => []);
    const activeId = getActiveId();
    select.innerHTML = connections.length
      ? connections.map((c) => `<option value="${c.id}" ${c.id === activeId ? "selected" : ""}>${escapeHtml(c.name)} (${escapeHtml(c.username)} @ ${escapeHtml(c.url)})</option>`).join("")
      : `<option value="">（暂无连接）</option>`;
    syncDisconnectVisibility();
  }

  async function doConnect(id: string) {
    const profile = await api.getConnection(id);
    if (!profile) return;
    await api.connect(id);
    events.onConnect(id, profile);
    syncDisconnectVisibility();
  }

  select.addEventListener("change", async () => {
    const id = select.value;
    if (!id) return;
    try {
      await doConnect(id);
    } catch (e) {
      alert("连接失败：" + e);
      loadSelect();
    }
  });

  disconnectBtn.addEventListener("click", async () => {
    const id = getActiveId();
    if (!id) return;
    if (!confirm("确定要断开当前连接吗？")) return;
    try {
      await api.disconnect(id);
      events.onDisconnect();
      syncDisconnectVisibility();
      loadSelect();
    } catch (e) {
      alert("断开失败：" + e);
    }
  });

  root.querySelector("#conn-manage")!.addEventListener("click", () =>
    showManageModal(root, async () => { await loadSelect(); }, async (id) => {
      try { await doConnect(id); } catch (e) { alert("连接失败：" + e); }
    }),
  );
  root.querySelector("#conn-add")!.addEventListener("click", () => {
    form.style.display = form.style.display === "none" ? "grid" : "none";
    urlInput.focus();
  });
  root.querySelector("#cfg-connect")!.addEventListener("click", async () => {
    const url = urlInput.value.trim();
    const username = userInput.value.trim();
    const password = passInput.value;
    if (!url) return alert("请输入服务器地址");
    const cfg = { url, username, password };
    const btn = root.querySelector<HTMLButtonElement>("#cfg-connect")!;
    btn.disabled = true;
    try {
      await api.testConnection(cfg);
      const profile = await api.saveConnection(cfg);
      passInput.value = "";
      form.style.display = "none";
      urlInput.value = "";
      userInput.value = "";
      await loadSelect();
      await api.connect(profile.id);
      events.onConnect(profile.id, profile);
      syncDisconnectVisibility();
    } catch (e) {
      alert("连接失败：" + e);
    } finally {
      btn.disabled = false;
    }
  });

  loadSelect();
  return { refresh: loadSelect };
}

async function showManageModal(anchor: HTMLElement, refreshDone: () => void, onConnect: (id: string) => Promise<void>) {
  const connections = await api.listConnections().catch(() => [] as ConnectionSummary[]);
  const backdrop = document.createElement("div");
  backdrop.className = "modal-backdrop";
  backdrop.innerHTML = `
    <div class="modal" style="min-width:520px">
      <h3>连接管理</h3>
      <div class="conn-list" id="manage-list"></div>
      <div class="row">
        <button class="ghost" id="manage-add">新增连接</button>
        <button class="ghost" id="manage-close">关闭</button>
      </div>
    </div>`;
  document.body.appendChild(backdrop);
  const listEl = backdrop.querySelector("#manage-list")!;
  const close = () => backdrop.remove();
  backdrop.addEventListener("click", (e) => { if (e.target === backdrop) close(); });
  backdrop.querySelector("#manage-close")!.addEventListener("click", close);
  backdrop.querySelector("#manage-add")!.addEventListener("click", () => {
    close();
    (anchor.querySelector("#conn-add") as HTMLElement)?.click();
  });

  function renderList(list: ConnectionSummary[]) {
    if (!list.length) {
      listEl.innerHTML = `<div class="empty">暂无连接，点击下方「新增连接」添加</div>`;
      return;
    }
    listEl.innerHTML = list
      .map(
        (c) => `
        <div class="conn-item" data-id="${c.id}">
          <span class="meta">
            <div class="name">${escapeHtml(c.name)}</div>
            <div class="url">${escapeHtml(c.url)}</div>
            <div class="last">用户 ${escapeHtml(c.username)} · 上次 ${formatRelativeTime(c.lastUsed)}</div>
          </span>
          <span class="actions">
            <button class="ghost small" data-act="edit">编辑</button>
            <button class="ghost small danger" data-act="del">删除</button>
          </span>
        </div>`
      )
      .join("");
    listEl.querySelectorAll(".conn-item").forEach((item) => {
      const id = (item as HTMLElement).dataset.id!;
      item.addEventListener("click", async (e) => {
        if ((e.target as HTMLElement).closest("button")) return;
        try {
          await onConnect(id);
          close();
          refreshDone();
        } catch (err) {
          alert("连接失败：" + err);
        }
      });
      item.querySelector("[data-act=edit]")!.addEventListener("click", async () => {
        const profile = await api.getConnection(id);
        if (!profile) return;
        await showEditModal(profile, async () => { renderList(await api.listConnections().catch(() => [])); });
      });
      item.querySelector("[data-act=del]")!.addEventListener("click", async () => {
        if (!confirm("确定删除该连接？")) return;
        await api.deleteConnection(id);
        renderList(await api.listConnections().catch(() => []));
        refreshDone();
      });
    });
  }
  renderList(connections);
}

async function showEditModal(profile: ConnectionProfile, done: () => void) {
  const backdrop = document.createElement("div");
  backdrop.className = "modal-backdrop";
  backdrop.innerHTML = `
    <div class="modal" style="min-width:420px">
      <h3>编辑连接</h3>
      <div class="field"><label>名称</label><input id="e-name" value="${escapeHtml(profile.name)}" /></div>
      <div class="field"><label>服务器地址</label><input id="e-url" value="${escapeHtml(profile.url)}" /></div>
      <div class="field"><label>用户名</label><input id="e-user" value="${escapeHtml(profile.username)}" /></div>
      <div class="field"><label>密码</label><input id="e-pass" type="password" value="${escapeHtml(profile.password)}" /></div>
      <div class="row">
        <button class="ghost" id="e-cancel">取消</button>
        <button id="e-test" class="ghost">测试连接</button>
        <button id="e-save">保存</button>
      </div>
    </div>`;
  document.body.appendChild(backdrop);
  const close = () => backdrop.remove();
  backdrop.querySelector("#e-cancel")!.addEventListener("click", close);
  backdrop.addEventListener("click", (e) => { if (e.target === backdrop) close(); });
  backdrop.querySelector("#e-save")!.addEventListener("click", async () => {
    const name = backdrop.querySelector<HTMLInputElement>("#e-name")!.value.trim();
    const url = backdrop.querySelector<HTMLInputElement>("#e-url")!.value.trim();
    const username = backdrop.querySelector<HTMLInputElement>("#e-user")!.value.trim();
    const password = backdrop.querySelector<HTMLInputElement>("#e-pass")!.value;
    if (!url) return alert("请输入地址");
    try {
      await api.saveConnection({ url, username, password }, profile.id, name || undefined);
      close();
      done();
    } catch (e) { alert("保存失败：" + e); }
  });
  backdrop.querySelector("#e-test")!.addEventListener("click", async () => {
    const cfg = {
      url: backdrop.querySelector<HTMLInputElement>("#e-url")!.value.trim(),
      username: backdrop.querySelector<HTMLInputElement>("#e-user")!.value.trim(),
      password: backdrop.querySelector<HTMLInputElement>("#e-pass")!.value,
    };
    try { await api.testConnection(cfg); alert("连接正常"); }
    catch (e) { alert("连接失败：" + e); }
  });
}

function escapeHtml(s: string) {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
}
