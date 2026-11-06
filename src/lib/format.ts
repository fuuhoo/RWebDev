export function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 2)} ${units[i]}`;
}

export function formatDate(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  return d.toLocaleString("zh-CN", { hour12: false });
}

export function formatRelativeTime(ts: number | null): string {
  if (!ts) return "从未";
  const d = new Date(ts);
  return d.toLocaleString("zh-CN", { hour12: false });
}

export function iconFor(name: string, isDir: boolean): string {
  if (isDir) return "📁";
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"].includes(ext)) return "🖼️";
  if (["mp4", "webm", "mov", "mkv"].includes(ext)) return "🎬";
  if (["mp3", "wav", "ogg", "flac"].includes(ext)) return "🎵";
  if (ext === "pdf") return "📕";
  if (["zip", "rar", "7z", "tar", "gz"].includes(ext)) return "📦";
  if (["txt", "md", "log", "json", "xml", "css", "js", "ts", "rs", "py"].includes(ext)) return "📄";
  return "📄";
}
