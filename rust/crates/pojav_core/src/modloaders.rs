use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModSearchItem {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
}

pub fn search_modrinth(query: &str, limit: usize) -> Result<Vec<ModSearchItem>, String> {
    let url = format!(
        "https://api.modrinth.com/v2/search?query={}&limit={}",
        query.replace(' ', "%20"),
        limit.clamp(1, 50)
    );
    let resp: serde_json::Value = ureq::get(&url)
        .set("User-Agent", "HuTaoLauncher-Rust/0.1.0")
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())?;

    let mut items = Vec::new();
    if let Some(hits) = resp["hits"].as_array() {
        for hit in hits {
            items.push(ModSearchItem {
                project_id: hit["project_id"].as_str().unwrap_or_default().to_string(),
                title: hit["title"].as_str().unwrap_or_default().to_string(),
                description: hit["description"].as_str().unwrap_or_default().to_string(),
                downloads: hit["downloads"].as_u64().unwrap_or(0),
                icon_url: hit["icon_url"].as_str().map(ToString::to_string),
            });
        }
    }
    Ok(items)
}
