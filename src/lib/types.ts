export interface DavConfig { url: string; username: string; password: string; }

export interface ConnectionProfile {
  id: string;
  name: string;
  url: string;
  username: string;
  password: string;
  lastUsed: number | null;
}

export interface ConnectionSummary {
  id: string;
  name: string;
  url: string;
  username: string;
  lastUsed: number | null;
}

export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  modified: string | null;
  contentType: string | null;
}

export interface ProgressEvent { transferId: string; fileName: string; uploaded: number; total: number; }

export interface PreviewData {
  name: string;
  contentType: string;
  kind: "image" | "video" | "audio" | "pdf" | "text" | "binary";
  dataBase64: string;
  size: number;
}
