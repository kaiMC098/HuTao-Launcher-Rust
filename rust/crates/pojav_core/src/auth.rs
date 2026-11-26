use crate::models::MinecraftAccount;
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid player username")]
    InvalidUsername,
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("Authentication response missing expected field: {0}")]
    InvalidResponse(String),
}

pub fn create_offline_account(username: &str) -> Result<MinecraftAccount, AuthError> {
    let trimmed = username.trim();
    if trimmed.is_empty() || trimmed.len() > 16 {
        return Err(AuthError::InvalidUsername);
    }
    let offline_seed = format!("OfflinePlayer:{}", trimmed);
    let uuid = Uuid::new_v3(&Uuid::NAMESPACE_DNS, offline_seed.as_bytes());

    Ok(MinecraftAccount {
        username: trimmed.to_string(),
        uuid: uuid.to_string(),
        access_token: "0".to_string(),
        xuid: "0".to_string(),
        client_token: Uuid::new_v4().to_string(),
        is_microsoft: false,
        expires_at_unix: 0,
    })
}

pub fn authenticate_microsoft_token(ms_access_token: &str) -> Result<MinecraftAccount, AuthError> {
    let xbl_res: serde_json::Value = ureq::post("https://user.auth.xboxlive.com/user/authenticate")
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .send_json(json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", ms_access_token)
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .map_err(|e| AuthError::Http(e.to_string()))?
        .into_json()
        .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

    let xbl_token = xbl_res["Token"]
        .as_str()
        .ok_or_else(|| AuthError::InvalidResponse("Missing XBL Token".into()))?;
    let user_hash = xbl_res["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or_else(|| AuthError::InvalidResponse("Missing XBL uhs".into()))?;

    let xsts_res: serde_json::Value = ureq::post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .send_json(json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl_token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .map_err(|e| AuthError::Http(e.to_string()))?
        .into_json()
        .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

    let xsts_token = xsts_res["Token"]
        .as_str()
        .ok_or_else(|| AuthError::InvalidResponse("Missing XSTS Token".into()))?;

    let mc_res: serde_json::Value =
        ureq::post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .set("Content-Type", "application/json")
            .send_json(json!({
                "identityToken": format!("XBL3.0 x={};{}", user_hash, xsts_token)
            }))
            .map_err(|e| AuthError::Http(e.to_string()))?
            .into_json()
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

    let mc_access_token = mc_res["access_token"]
        .as_str()
        .ok_or_else(|| AuthError::InvalidResponse("Missing MC access_token".into()))?;

    let profile_res: serde_json::Value =
        ureq::get("https://api.minecraftservices.com/minecraft/profile")
            .set("Authorization", &format!("Bearer {}", mc_access_token))
            .call()
            .map_err(|e| AuthError::Http(e.to_string()))?
            .into_json()
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

    let uuid = profile_res["id"]
        .as_str()
        .ok_or_else(|| AuthError::InvalidResponse("Account does not own Minecraft Java".into()))?;
    let username = profile_res["name"]
        .as_str()
        .ok_or_else(|| AuthError::InvalidResponse("Missing Minecraft profile name".into()))?;

    Ok(MinecraftAccount {
        username: username.to_string(),
        uuid: uuid.to_string(),
        access_token: mc_access_token.to_string(),
        xuid: user_hash.to_string(),
        client_token: Uuid::new_v4().to_string(),
        is_microsoft: true,
        expires_at_unix: 0,
    })
}
