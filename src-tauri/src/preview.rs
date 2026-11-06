use crate::dav::{join_url, DavState};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Serialize;
use tauri::State;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PreviewData {
    pub name: String,
    pub content_type: String,
    pub kind: String,
    pub data_base64: String,
    pub size: u64,
}

#[tauri::command]
pub async fn preview_file(state: State<'_, DavState>, path: String, connection_id: Option<String>) -> Result<PreviewData, String> {
    crate::dav::ensure_loaded(&state).await;
    let (cfg, _id) = {
        let id = if let Some(id) = connection_id.clone() {
            id
        } else if let Some(s) = state.list_summaries().await.into_iter().max_by_key(|s| s.last_used.unwrap_or(0)) {
            s.id
        } else {
            return Err("尚未连接服务器".to_string());
        };
        let cfg = if let Some(c) = state.get_session(&id).await {
            c
        } else if let Some(p) = state.get_profile(&id).await {
            crate::dav::DavConfig { url: p.url, username: p.username, password: p.password }
        } else {
            return Err("尚未连接服务器".to_string());
        };
        (cfg, id)
    };
    let url = join_url(&cfg.url, &path);
    let bytes = crate::dav::http_get(&cfg, &url).await?;
    let name = path.rsplit('/').next().unwrap_or("file").to_string();
    let kind = classify(&name);
    let content_type = mime_guess(&name);
    if kind == "text" {
        let text = String::from_utf8_lossy(&bytes).into_owned();
        Ok(PreviewData { name, content_type: "text/plain; charset=utf-8".into(), kind, data_base64: STANDARD.encode(text.as_bytes()), size: bytes.len() as u64 })
    } else {
        Ok(PreviewData { name, content_type, kind, data_base64: STANDARD.encode(&bytes), size: bytes.len() as u64 })
    }
}

fn classify(name: &str) -> String {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png"|"jpg"|"jpeg"|"gif"|"webp"|"bmp"|"svg" => "image".into(),
        "mp4"|"webm"|"mov" => "video".into(),
        "mp3"|"wav"|"ogg" => "audio".into(),
        "pdf" => "pdf".into(),
        "txt"|"md"|"json"|"xml"|"html"|"htm"|"css"|"js"|"ts"|"rs"|"py"|"log"|"yaml"|"yml"|"toml"|"csv" => "text".into(),
        _ => "binary".into(),
    }
}
fn mime_guess(name: &str) -> String {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "png"=>"image/png".into(),"jpg"|"jpeg"=>"image/jpeg".into(),"gif"=>"image/gif".into(),"webp"=>"image/webp".into(),
        "svg"=>"image/svg+xml".into(),"mp4"=>"video/mp4".into(),"webm"=>"video/webm".into(),"mp3"=>"audio/mpeg".into(),
        "pdf"=>"application/pdf".into(), _=>"application/octet-stream".into(),
    }
}
