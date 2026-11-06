use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DavConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub url: String,
    pub username: String,
    pub password: String,
    pub last_used: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSummary {
    pub id: String,
    pub name: String,
    pub url: String,
    pub username: String,
    pub last_used: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Clone, Default)]
pub struct DavState {
    pub profiles: Arc<RwLock<Vec<ConnectionProfile>>>,
    pub sessions: Arc<RwLock<HashMap<String, DavConfig>>>,
    pub clients: Arc<RwLock<HashMap<String, reqwest::Client>>>,
    pub disconnected: Arc<RwLock<HashSet<String>>>,
}

impl DavState {
    pub async fn get_profile(&self, id: &str) -> Option<ConnectionProfile> {
        self.profiles.read().await.iter().find(|p| p.id == id).cloned()
    }
    pub async fn get_session(&self, id: &str) -> Option<DavConfig> {
        self.sessions.read().await.get(id).cloned()
    }
    pub async fn get_client(&self, id: &str, cfg: &DavConfig) -> reqwest::Client {
        if let Some(c) = self.clients.read().await.get(id).cloned() {
            return c;
        }
        let c = build_client(cfg);
        self.clients.write().await.insert(id.to_string(), c.clone());
        c
    }
    pub async fn upsert_profile(&self, cfg: DavConfig, id: Option<String>, name: Option<String>) -> ConnectionProfile {
        let now = now_millis();
        let mut profiles = self.profiles.write().await;
        let existing = id.as_ref().and_then(|id| profiles.iter().position(|p| &p.id == id));
        let profile = if let Some(i) = existing {
            let p = &mut profiles[i];
            p.url = cfg.url.clone();
            p.username = cfg.username.clone();
            p.password = cfg.password.clone();
            if let Some(n) = name { p.name = n; }
            p.last_used = Some(now);
            profiles[i].clone()
        } else {
            let id = id.unwrap_or_else(new_id);
            let name = name.unwrap_or_else(|| {
                let u = cfg.url.trim_end_matches('/');
                u.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(u).to_string()
            });
            let p = ConnectionProfile { id, name, url: cfg.url.clone(), username: cfg.username.clone(), password: cfg.password.clone(), last_used: Some(now) };
            profiles.push(p.clone());
            p
        };
        drop(profiles);
        self.disconnected.write().await.remove(&profile.id);
        self.sessions.write().await.insert(profile.id.clone(), cfg);
        save_persist(&self.profiles.read().await);
        profile
    }
    pub async fn touch_last_used(&self, id: &str) {
        if let Some(p) = self.profiles.write().await.iter_mut().find(|p| p.id == id) {
            p.last_used = Some(now_millis());
        }
        save_persist(&self.profiles.read().await);
    }
    pub async fn list_summaries(&self) -> Vec<ConnectionSummary> {
        self.profiles.read().await.iter().map(|p| ConnectionSummary { id: p.id.clone(), name: p.name.clone(), url: p.url.clone(), username: p.username.clone(), last_used: p.last_used }).collect()
    }
    pub async fn remove_profile(&self, id: &str) -> bool {
        let mut profiles = self.profiles.write().await;
        let before = profiles.len();
        profiles.retain(|p| p.id != id);
        let removed = profiles.len() != before;
        drop(profiles);
        if removed {
            self.sessions.write().await.remove(id);
            self.clients.write().await.remove(id);
            save_persist(&self.profiles.read().await);
        }
        removed
    }
}

pub fn now_millis() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}
fn new_id() -> String {
    format!("c{:x}-{:x}", now_millis(), std::process::id())
}

const LEGACY_PERSIST: &str = "webdav_config.json";
const PERSIST: &str = "webdav_connections.json";

fn persist_path() -> std::path::PathBuf {
    dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("webdav-client").join(PERSIST)
}
fn legacy_persist_path() -> std::path::PathBuf {
    dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("webdav-client").join(LEGACY_PERSIST)
}
fn save_persist(profiles: &[ConnectionProfile]) {
    let path = persist_path();
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(s) = serde_json::to_string_pretty(profiles) {
        let _ = std::fs::write(path, s);
    }
}
fn load_persist() -> Vec<ConnectionProfile> {
    let path = persist_path();
    if let Ok(s) = std::fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<Vec<ConnectionProfile>>(&s) {
            return v;
        }
        if let Ok(v) = serde_json::from_str::<ConnectionProfile>(&s) {
            return vec![v];
        }
    }
    if let Ok(s) = std::fs::read_to_string(legacy_persist_path()) {
        if let Ok(cfg) = serde_json::from_str::<DavConfig>(&s) {
            let p = ConnectionProfile { id: new_id(), name: cfg.url.trim_end_matches('/').rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(&cfg.url).to_string(), url: cfg.url, username: cfg.username, password: cfg.password, last_used: Some(now_millis()) };
            return vec![p];
        }
    }
    Vec::new()
}

pub async fn ensure_loaded(state: &DavState) {
    if state.profiles.read().await.is_empty() {
        let loaded = load_persist();
        if !loaded.is_empty() {
            let mut profiles = state.profiles.write().await;
            if profiles.is_empty() {
                *profiles = loaded;
            }
        }
    }
}

/// WebDAV base URL from the connection URL. Handles URLs that contain a path
/// (e.g. http://host:5244/dav or http://host/dav/webdav) so uploaded files
/// land at {base}/{path}/{name} instead of appending to the base URL itself.
pub fn dav_root_base(connection_url: &str, _upload_root: Option<&str>) -> String {
    let b = connection_url.trim_end_matches('/').to_string();
    b
}

fn join_upload_url(base: &str, _upload_root: Option<&str>, name: &str) -> String {
    if base.is_empty() {
        return format!("/{}", name);
    }
    join_url(base, name)
}

fn join_upload_name(base: &str, upload_path: Option<&str>, name: &str) -> String {
    let base = base.trim_end_matches('/');
    let p = upload_path.unwrap_or("").trim_matches('/');
    if p.is_empty() {
        return join_url(base, name);
    }
    // upload_path is relative to the DAV root (e.g. "webdav" or "/webdav")
    let joined = if name.is_empty() { p.to_string() } else { format!("{p}/{name}") };
    join_url(base, &joined)
}

pub fn join_url(base: &str, sub: &str) -> String {
    let b = base.trim_end_matches('/');
    let s = sub.trim_start_matches('/');
    if s.is_empty() {
        return format!("{b}/");
    }
    let enc: String = s
        .split('/')
        .map(|seg| {
            if seg.is_empty() {
                String::new()
            } else {
                utf8_percent_encode(seg, NON_ALPHANUMERIC).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/");
    format!("{b}/{enc}")
}

fn build_client(cfg: &DavConfig) -> reqwest::Client {
    let mut b = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));
    if !cfg.username.is_empty() {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let token = STANDARD.encode(format!("{}:{}", cfg.username, cfg.password));
        b = b.default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert(reqwest::header::AUTHORIZATION, format!("Basic {token}").parse().unwrap());
            h
        });
    }
    b.build().unwrap_or_else(|_| reqwest::Client::new())
}

async fn resolve_cfg(state: &DavState, connection_id: Option<String>) -> Result<(String, DavConfig), String> {
    ensure_loaded(state).await;
    if let Some(id) = connection_id {
        if state.disconnected.read().await.contains(&id) {
            return Err("not connected".to_string());
        }
        if let Some(cfg) = state.get_session(&id).await {
            return Ok((id, cfg));
        }
        if let Some(p) = state.get_profile(&id).await {
            let cfg = DavConfig { url: p.url.clone(), username: p.username.clone(), password: p.password.clone() };
            state.sessions.write().await.insert(id.clone(), cfg.clone());
            return Ok((id, cfg));
        }
        return Err("connection not found".to_string());
    }
    let disconnected = state.disconnected.read().await.clone();
    for s in state.list_summaries().await.into_iter().filter(|s| !disconnected.contains(&s.id)) {
        if let Some(cfg) = state.get_session(&s.id).await {
            return Ok((s.id, cfg));
        }
        if let Some(p) = state.get_profile(&s.id).await {
            let cfg = DavConfig { url: p.url.clone(), username: p.username.clone(), password: p.password.clone() };
            state.sessions.write().await.insert(s.id.clone(), cfg.clone());
            return Ok((s.id, cfg));
        }
    }
    Err("not connected".to_string())
}

// ---- Connection management commands ----

#[tauri::command]
pub async fn list_connections(state: State<'_, DavState>) -> Result<Vec<ConnectionSummary>, String> {
    ensure_loaded(&state).await;
    Ok(state.list_summaries().await)
}

#[tauri::command]
pub async fn get_connection(state: State<'_, DavState>, id: String) -> Result<Option<ConnectionProfile>, String> {
    ensure_loaded(&state).await;
    Ok(state.get_profile(&id).await)
}

#[tauri::command]
pub async fn save_connection(state: State<'_, DavState>, config: DavConfig, id: Option<String>, name: Option<String>) -> Result<ConnectionProfile, String> {
    ensure_loaded(&state).await;
    Ok(state.upsert_profile(config, id, name).await)
}

#[tauri::command]
pub async fn delete_connection(state: State<'_, DavState>, id: String) -> Result<bool, String> {
    ensure_loaded(&state).await;
    Ok(state.remove_profile(&id).await)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, DavState>, id: String) -> Result<bool, String> {
    ensure_loaded(&state).await;
    state.sessions.write().await.remove(&id);
    state.clients.write().await.remove(&id);
    state.disconnected.write().await.insert(id.clone());
    Ok(true)
}

#[tauri::command]
pub async fn test_connection(state: State<'_, DavState>, config: DavConfig) -> Result<bool, String> {
    ensure_loaded(&state).await;
    let c = build_client(&config);
    let resp = c
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), join_url(&config.url, ""))
        .header("Depth", "0")
        .body("<?xml version=\"1.0\"?><propfind xmlns=\"DAV:\"><prop><displayname/></prop></propfind>")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let code = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    let is_xml = !body.trim_start().is_empty() && body.trim_start().starts_with('<') && !body.to_lowercase().starts_with("<!doctype") && !body.to_lowercase().contains("<html");
    if ((200..300).contains(&code) || code == 207) && is_xml {
        Ok(true)
    } else if (200..300).contains(&code) && !is_xml {
        Err("服务器返回的不是 WebDAV XML（HTTP 200 但返回了网页）。请检查连接地址是否为 WebDAV 端点，AList 需用 /dav/ 地址而非 /webdav。".to_string())
    } else {
        Err(format!("HTTP {}", code))
    }
}

#[tauri::command]
pub async fn connect(state: State<'_, DavState>, id: String) -> Result<bool, String> {
    ensure_loaded(&state).await;
    let p = state.get_profile(&id).await.ok_or("connection not found")?;
    let cfg = DavConfig { url: p.url.clone(), username: p.username.clone(), password: p.password.clone() };
    let c = build_client(&cfg);
    let resp = c
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), join_url(&cfg.url, ""))
        .header("Depth", "0")
        .body("<?xml version=\"1.0\"?><propfind xmlns=\"DAV:\"><prop><displayname/></prop></propfind>")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let code = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    let is_xml = !body.trim_start().is_empty() && body.trim_start().starts_with('<') && !body.to_lowercase().starts_with("<!doctype") && !body.to_lowercase().contains("<html");
    if ((200..300).contains(&code) || code == 207) && is_xml {
        state.disconnected.write().await.remove(&id);
        state.sessions.write().await.insert(id.clone(), cfg.clone());
        state.clients.write().await.insert(id.clone(), c);
        state.touch_last_used(&id).await;
        Ok(true)
    } else if (200..300).contains(&code) && !is_xml {
        Err("服务器返回的不是 WebDAV XML（HTTP 200 但返回了网页）。请检查连接地址是否为 WebDAV 端点，AList 需用 /dav/ 地址而非 /webdav。".to_string())
    } else {
        Err(format!("HTTP {}", code))
    }
}

// ---- Backward-compat single-config commands ----

#[tauri::command]
pub async fn set_config(state: State<'_, DavState>, config: DavConfig) -> Result<ConnectionProfile, String> {
    ensure_loaded(&state).await;
    let profile = state.upsert_profile(config, None, None).await;
    Ok(profile)
}

#[tauri::command]
pub async fn get_config(state: State<'_, DavState>, connection_id: Option<String>) -> Result<Option<DavConfig>, String> {
    ensure_loaded(&state).await;
    let (_, cfg) = resolve_cfg(&state, connection_id).await?;
    Ok(Some(cfg))
}

fn local(tag: &str) -> &str {
    tag.rsplit(':').next().unwrap_or(tag)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn href_to_path(href: &str) -> String {
    let h = href.split('?').next().unwrap_or(href).split('#').next().unwrap_or(href);
    let p = if h.starts_with("http://") || h.starts_with("https://") {
        url::Url::parse(h).map(|u| u.path().to_string()).unwrap_or_else(|_| h.to_string())
    } else {
        h.to_string()
    };
    let decoded = percent_decode(&p);
    let trimmed = decoded.trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed
    }
}

fn parse_propfind(text: &str, base_path: &str) -> Result<Vec<FileEntry>, String> {
    if !text.trim_start().starts_with('<') {
        return Err("服务器返回的不是 WebDAV XML（HTTP 200 但返回了网页）。请检查连接地址是否为 WebDAV 端点，AList 需用 /dav/ 地址而非 /webdav。".to_string());
    }
    let base_norm = base_path.trim_end_matches('/').to_string();
    let mut r = Reader::from_str(text);
    let mut buf = vec![];
    let mut cur: Option<FileEntry> = None;
    let mut field: Option<String> = None;
    let mut out: Vec<FileEntry> = Vec::new();
    loop {
        match r.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let t = local(&tag).to_string();
                match t.as_str() {
                    "href" => field = Some("href".into()),
                    "displayname" => field = Some("displayname".into()),
                    "getcontentlength" => field = Some("len".into()),
                    "getlastmodified" => field = Some("modified".into()),
                    "getcontenttype" => field = Some("ctype".into()),
                    "collection" => {
                        if let Some(c) = &mut cur {
                            c.is_dir = true;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(f) = &field {
                    let v = e.unescape().unwrap_or_default().to_string();
                    match f.as_str() {
                        "href" => {
                            let p = href_to_path(&v);
                            cur = Some(FileEntry { name: String::new(), path: p, is_dir: false, size: 0, modified: None, content_type: None });
                        }
                        "displayname" => {
                            if let Some(c) = &mut cur {
                                c.name = v;
                            }
                        }
                        "len" => {
                            if let Some(c) = &mut cur {
                                c.size = v.parse().unwrap_or(0);
                            }
                        }
                        "modified" => {
                            if let Some(c) = &mut cur {
                                c.modified = Some(v);
                            }
                        }
                        "ctype" => {
                            if let Some(c) = &mut cur {
                                c.content_type = Some(v);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let t = local(&tag).to_string();
                if t == "response" {
                    if let Some(mut c) = cur.take() {
                        if c.name.is_empty() {
                            c.name = c.path.rsplit('/').filter(|s| !s.is_empty()).next().unwrap_or(&c.path).to_string();
                        }
                        if c.path.trim_end_matches('/') != base_norm {
                            out.push(c);
                        }
                    }
                }
                if ["href", "displayname", "getcontentlength", "getlastmodified", "getcontenttype"].contains(&t.as_str()) {
                    field = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.to_string()),
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

pub async fn http_get(cfg: &DavConfig, url: &str) -> Result<Vec<u8>, String> {
    let c = build_client(cfg);
    let r = c.get(url).send().await.map_err(|e| e.to_string())?;
    if !r.status().is_success() {
        return Err(format!("HTTP {}", r.status()));
    }
    Ok(r.bytes().await.map_err(|e| e.to_string())?.to_vec())
}

async fn cfg_and_client(state: &DavState, connection_id: Option<String>) -> Result<(String, DavConfig, reqwest::Client), String> {
    let (id, cfg) = resolve_cfg(state, connection_id).await?;
    let c = state.get_client(&id, &cfg).await;
    Ok((id, cfg, c))
}

#[tauri::command]
pub async fn list_dir(state: State<'_, DavState>, path: String, connection_id: Option<String>) -> Result<Vec<FileEntry>, String> {
    let (_id, cfg, c) = cfg_and_client(&state, connection_id).await?;
    let url = join_url(&cfg.url, &path);
    let resp = c
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
        .header("Depth", "1")
        .body("<?xml version=\"1.0\"?><propfind xmlns=\"DAV:\"><prop><displayname/><getcontentlength/><getlastmodified/><getcontenttype/><resourcetype/></prop></propfind>")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() && resp.status().as_u16() != 207 {
        return Err(format!("HTTP {}", resp.status()));
    }
    let is_xml = resp
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or("").contains("xml"))
        .unwrap_or(false);
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !is_xml && !text.trim_start().starts_with('<') {
        return Err("服务器返回的不是 WebDAV XML（HTTP 200 但返回了网页）。请检查连接地址是否为 WebDAV 端点，AList 需用 /dav/ 地址而非 /webdav。".to_string());
    }
    let norm = if path.is_empty() { "/".to_string() } else { path.clone() };
    let mut entries = parse_propfind(&text, &norm)?;
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(entries)
}

#[tauri::command]
pub async fn create_folder(state: State<'_, DavState>, path: String, connection_id: Option<String>) -> Result<(), String> {
    let (_id, cfg, c) = cfg_and_client(&state, connection_id).await?;
    let url = join_url(&cfg.url, &path);
    let r = c.request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url).send().await.map_err(|e| e.to_string())?;
    if r.status().is_success() || r.status().as_u16() == 405 {
        Ok(())
    } else {
        Err(format!("HTTP {}", r.status()))
    }
}

#[tauri::command]
pub async fn delete_entry(state: State<'_, DavState>, path: String, connection_id: Option<String>) -> Result<(), String> {
    let (_id, cfg, c) = cfg_and_client(&state, connection_id).await?;
    let url = join_url(&cfg.url, &path);
    let r = c.delete(&url).send().await.map_err(|e| e.to_string())?;
    if r.status().is_success() || r.status().as_u16() == 200 || r.status().as_u16() == 204 {
        Ok(())
    } else {
        Err(format!("HTTP {}", r.status()))
    }
}

#[tauri::command]
pub async fn rename_entry(state: State<'_, DavState>, old_path: String, new_name: String, connection_id: Option<String>) -> Result<(), String> {
    let (_id, cfg, c) = cfg_and_client(&state, connection_id).await?;
    let from_url = join_url(&cfg.url, &old_path);
    let parent = old_path.rsplit_once('/').map(|(p, _)| p.to_string()).unwrap_or_default();
    let dest_path = if parent.is_empty() { new_name.clone() } else { format!("{parent}/{new_name}") };
    let dest_url = join_url(&cfg.url, &dest_path);
    let r = c
        .request(reqwest::Method::from_bytes(b"MOVE").unwrap(), &from_url)
        .header("Destination", &dest_url)
        .header("Overwrite", "T")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if r.status().is_success() || r.status().as_u16() == 201 || r.status().as_u16() == 204 {
        Ok(())
    } else {
        Err(format!("HTTP {}", r.status()))
    }
}

#[tauri::command]
pub async fn download_file(state: State<'_, DavState>, path: String, connection_id: Option<String>) -> Result<String, String> {
    let (_id, cfg, _c) = cfg_and_client(&state, connection_id).await?;
    let bytes = http_get(&cfg, &join_url(&cfg.url, &path)).await?;
    let name = path.rsplit('/').next().unwrap_or("download").to_string();
    let save = dirs::download_dir().map(|d| d.join(&name)).unwrap_or_else(|| std::path::PathBuf::from(&name));
    if let Some(p) = save.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    std::fs::write(&save, &bytes).map_err(|e| e.to_string())?;
    Ok(save.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn upload_file(state: State<'_, DavState>, name: String, data: Vec<u8>, connection_id: Option<String>, upload_root: Option<String>, upload_path: Option<String>) -> Result<String, String> {
    let (_id, cfg, c) = cfg_and_client(&state, connection_id).await?;
    let base = dav_root_base(&cfg.url, upload_root.as_deref());
    let url = join_upload_name(&base, upload_path.as_deref(), &name);
    let r = c.put(&url).body(data).send().await.map_err(|e| e.to_string())?;
    if r.status().is_success() || r.status().as_u16() == 201 || r.status().as_u16() == 204 {
        Ok(url)
    } else {
        Err(format!("HTTP {}", r.status()))
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub transfer_id: String,
    pub file_name: String,
    pub uploaded: u64,
    pub total: u64,
}

#[tauri::command]
pub async fn upload_local_file(app: AppHandle, state: State<'_, DavState>, local_path: String, transfer_id: String, connection_id: Option<String>, upload_root: Option<String>, upload_path: Option<String>) -> Result<String, String> {
    let (id, cfg, c) = cfg_and_client(&state, connection_id).await?;
    let lp = std::path::PathBuf::from(&local_path);
    let total = std::fs::metadata(&lp).map(|m| m.len()).unwrap_or(0);
    let name = lp.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "file".into());
    let base = dav_root_base(&cfg.url, upload_root.as_deref());
    let url = join_upload_name(&base, upload_path.as_deref(), &name);
    let _ = &id;
    let bytes = tokio::fs::read(&lp).await.map_err(|e| e.to_string())?;
    let _ = app.emit("upload-progress", ProgressEvent { transfer_id: transfer_id.clone(), file_name: name.clone(), uploaded: bytes.len() as u64, total });
    let r = c.put(&url).body(bytes).send().await.map_err(|e| e.to_string())?;
    if r.status().is_success() || r.status().as_u16() == 201 || r.status().as_u16() == 204 {
        let _ = app.emit("upload-progress", ProgressEvent { transfer_id: transfer_id.clone(), file_name: name.clone(), uploaded: total, total });
        Ok(url)
    } else {
        Err(format!("HTTP {}", r.status()))
    }
}
