import { invoke } from "@tauri-apps/api/core";
import type { DavConfig, ConnectionProfile, ConnectionSummary, FileEntry, PreviewData, ProgressEvent } from "./types";

export const api = {
  testConnection: (cfg: DavConfig) => invoke<boolean>("test_connection", { config: cfg }),
  listDir: (path: string, connectionId?: string) => invoke<FileEntry[]>("list_dir", { path, connectionId: connectionId ?? null }),
  createFolder: (path: string, connectionId?: string) => invoke<void>("create_folder", { path, connectionId: connectionId ?? null }),
  deleteEntry: (path: string, connectionId?: string) => invoke<void>("delete_entry", { path, connectionId: connectionId ?? null }),
  renameEntry: (oldPath: string, newName: string, connectionId?: string) => invoke<void>("rename_entry", { oldPath, newName, connectionId: connectionId ?? null }),
  downloadFile: (path: string, connectionId?: string) => invoke<string>("download_file", { path, connectionId: connectionId ?? null }),
  uploadBytes: (name: string, data: number[], connectionId?: string) => invoke<string>("upload_file", { name, data, connectionId: connectionId ?? null }),
  uploadLocalFile: (localPath: string, transferId: string, connectionId?: string, uploadPath?: string) => invoke<string>("upload_local_file", { localPath, transferId, connectionId: connectionId ?? null, uploadPath: uploadPath ?? null, uploadRoot: null }),
  setConfig: (config: DavConfig) => invoke<ConnectionProfile>("set_config", { config }),
  getConfig: () => invoke<DavConfig | null>("get_config"),
  previewFile: (path: string, connectionId?: string) => invoke<PreviewData>("preview_file", { path, connectionId: connectionId ?? null }),
  // Multi-connection management
  listConnections: () => invoke<ConnectionSummary[]>("list_connections"),
  getConnection: (id: string) => invoke<ConnectionProfile | null>("get_connection", { id }),
  saveConnection: (config: DavConfig, id?: string, name?: string) => invoke<ConnectionProfile>("save_connection", { config, id: id ?? null, name: name ?? null }),
  deleteConnection: (id: string) => invoke<boolean>("delete_connection", { id }),
  disconnect: (id: string) => invoke<boolean>("disconnect", { id }),
  connect: (id: string) => invoke<boolean>("connect", { id }),
  onProgress: (cb: (e: ProgressEvent) => void) => {
    return import("@tauri-apps/api/event").then(({ listen }) =>
      listen<ProgressEvent>("upload-progress", (ev) => cb(ev.payload))
    );
  },
};
