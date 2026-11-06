const LS_ACTIVE = "webdav.activeConnectionId";

export const storage = {
  getActiveConnectionId(): string | null {
    try { return localStorage.getItem(LS_ACTIVE); } catch { return null; }
  },
  setActiveConnectionId(id: string | null) {
    try {
      if (id) localStorage.setItem(LS_ACTIVE, id);
      else localStorage.removeItem(LS_ACTIVE);
    } catch {}
  },
};
