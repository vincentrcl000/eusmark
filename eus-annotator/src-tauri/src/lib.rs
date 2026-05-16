use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImageInfo {
    pub name: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CaseInfo {
    pub id: String,
    pub name: String,
    pub images: Vec<ImageInfo>,
    pub grade: Option<u32>,
    pub notes: String,
    pub annotated_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnnotationData {
    pub version: String,
    pub created_at: String,
    pub updated_at: String,
    pub root_path: String,
    pub cases: std::collections::HashMap<String, CaseInfo>,
}

#[tauri::command]
async fn get_last_folder() -> Result<Option<String>, String> {
    let doc_dir = dirs::document_dir().ok_or("Could not find document directory")?;
    let config_path = doc_dir.join("EUS标注工具").join("config.json");
    if config_path.exists() {
        let content = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
        let config: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(config.get("last_folder").and_then(|v| v.as_str()).map(|s| s.to_string()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn set_last_folder(path: String) -> Result<(), String> {
    let doc_dir = dirs::document_dir().ok_or("Could not find document directory")?;
    let app_dir = doc_dir.join("EUS标注工具");
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;
    }
    let config_path = app_dir.join("config.json");
    let config = serde_json::json!({ "last_folder": path });
    fs::write(config_path, config.to_string()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn scan_folder(path: String) -> Result<Vec<CaseInfo>, String> {
    let root = Path::new(&path);
    if !root.exists() {
        return Err("路径不存在".into());
    }

    let mut cases = Vec::new();
    
    let entries = fs::read_dir(root).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let case_name = entry.file_name().into_string().unwrap_or_default();
            let mut images = Vec::new();
            
            let img_entries = fs::read_dir(&path).map_err(|e| e.to_string())?;
            for img_entry in img_entries {
                let img_entry = img_entry.map_err(|e| e.to_string())?;
                let img_path = img_entry.path();
                if img_path.is_file() {
                    if let Some(ext) = img_path.extension().and_then(|s| s.to_str()) {
                        let ext = ext.to_lowercase();
                        if ["jpg", "jpeg", "png", "bmp", "webp"].contains(&ext.as_str()) {
                            images.push(ImageInfo {
                                name: img_entry.file_name().into_string().unwrap_or_default(),
                                path: img_entry.file_name().into_string().unwrap_or_default(),
                            });
                        }
                    }
                }
            }
            
            if !images.is_empty() {
                images.sort_by(|a, b| a.name.cmp(&b.name));
                cases.push(CaseInfo {
                    id: case_name.clone(),
                    name: case_name,
                    images,
                    grade: None,
                    notes: String::new(),
                    annotated_at: None,
                });
            }
        }
    }
    
    cases.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(cases)
}

#[tauri::command]
async fn load_annotations(folder_name: String) -> Result<Option<AnnotationData>, String> {
    let doc_dir = dirs::document_dir().ok_or("Could not find document directory")?;
    let annotations_dir = doc_dir.join("EUS标注工具").join("annotations");
    let file_path = annotations_dir.join(format!("{}.json", folder_name));
    
    if file_path.exists() {
        let content = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
        let data: AnnotationData = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(Some(data))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn save_annotations(folder_name: String, data: AnnotationData) -> Result<(), String> {
    let doc_dir = dirs::document_dir().ok_or("Could not find document directory")?;
    let annotations_dir = doc_dir.join("EUS标注工具").join("annotations");
    if !annotations_dir.exists() {
        fs::create_dir_all(&annotations_dir).map_err(|e| e.to_string())?;
    }
    let file_path = annotations_dir.join(format!("{}.json", folder_name));
    let content = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    fs::write(file_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn export_csv(data: AnnotationData, save_path: String) -> Result<(), String> {
    let mut content = String::from("\u{feff}"); // UTF-8 BOM for Excel
    content.push_str("病例ID,病例名称,分级,备注,标注时间,图片数量\n");
    
    let mut case_list: Vec<_> = data.cases.values().collect();
    case_list.sort_by(|a, b| a.name.cmp(&b.name));
    
    for c in case_list {
        let grade_str = c.grade.map(|g| g.to_string()).unwrap_or_else(|| "未标注".to_string());
        let notes = c.notes.replace("\"", "\"\"");
        content.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
            c.id,
            c.name,
            grade_str,
            notes,
            c.annotated_at.as_deref().unwrap_or(""),
            c.images.len()
        ));
    }
    
    fs::write(save_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .register_uri_scheme_protocol("asset", |_app, request| {
            let uri = request.uri().to_string();
            // 移除协议头
            let mut path = uri.strip_prefix("asset://localhost/").unwrap_or(&uri).to_string();
            
            // URL 解码 (处理中文和空格)
            path = urlencoding::decode(&path).unwrap_or(std::borrow::Cow::Borrowed(&path)).into_owned();

            // Windows 路径清理: 如果路径是 /C:/... 变成 C:/...
            if path.starts_with('/') && path.chars().nth(2) == Some(':') {
                path.remove(0);
            }

            match std::fs::read(&path) {
                Ok(data) => {
                    let mime = if path.ends_with(".png") { "image/png" } 
                              else if path.ends_with(".jpg") || path.ends_with(".jpeg") { "image/jpeg" }
                              else if path.ends_with(".webp") { "image/webp" }
                              else { "application/octet-stream" };
                    tauri::http::Response::builder()
                        .header("Content-Type", mime)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(data)
                        .unwrap()
                }
                Err(_) => tauri::http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap(),
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
        .setup(|_app| {
            if let Some(doc_dir) = dirs::document_dir() {
                let app_dir = doc_dir.join("EUS标注工具");
                if !app_dir.exists() {
                    let _ = std::fs::create_dir_all(&app_dir);
                }
                let annotations_dir = app_dir.join("annotations");
                if !annotations_dir.exists() {
                    let _ = std::fs::create_dir_all(&annotations_dir);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_last_folder,
            set_last_folder,
            scan_folder,
            load_annotations,
            save_annotations,
            export_csv
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}