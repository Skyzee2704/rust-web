//! Task verification. Default is the honor system (always true).
//!
//! X API endpoints for follows / liking users / retweeted_by need a paid tier and are
//! heavily rate-limited. If a check errors we fall back to `true` so real users aren't
//! blocked by API outages.

use serde::Deserialize;

use crate::config::{Task, TaskKind};

pub async fn verify_task(
    http: &reqwest::Client,
    task: &Task,
    x_user_id: &str,
    enabled: bool,
    bearer: &str,
) -> bool {
    if !enabled || task.target_id.is_none() || bearer.is_empty() {
        return true;
    }
    let target = task.target_id.unwrap_or("");
    let result = match task.kind {
        TaskKind::Follow => {
            check_in_list(
                http,
                bearer,
                &format!("users/{x_user_id}/following"),
                target,
            )
            .await
        }
        TaskKind::Like => {
            check_in_list(
                http,
                bearer,
                &format!("tweets/{target}/liking_users"),
                x_user_id,
            )
            .await
        }
        TaskKind::Retweet => {
            check_in_list(
                http,
                bearer,
                &format!("tweets/{target}/retweeted_by"),
                x_user_id,
            )
            .await
        }
        TaskKind::JoinDiscord | TaskKind::Custom => return true,
    };
    result.unwrap_or(true)
}

#[derive(Deserialize)]
struct XList {
    data: Option<Vec<XUser>>,
    meta: Option<XMeta>,
}

#[derive(Deserialize)]
struct XUser {
    id: String,
}

#[derive(Deserialize)]
struct XMeta {
    next_token: Option<String>,
}

async fn check_in_list(
    http: &reqwest::Client,
    bearer: &str,
    path: &str,
    needle: &str,
) -> Result<bool, ()> {
    let mut token: Option<String> = None;
    for _ in 0..3 {
        let mut url = format!("https://api.x.com/2/{path}?max_results=1000");
        if let Some(t) = &token {
            url.push_str("&pagination_token=");
            url.push_str(&urlencoding::encode(t));
        }
        let res = http
            .get(&url)
            .bearer_auth(bearer)
            .send()
            .await
            .map_err(|_| ())?;
        if !res.status().is_success() {
            return Err(());
        }
        let body: XList = res.json().await.map_err(|_| ())?;
        if body.data.unwrap_or_default().iter().any(|u| u.id == needle) {
            return Ok(true);
        }
        token = body
            .meta
            .and_then(|m| m.next_token)
            .filter(|t| !t.is_empty());
        if token.is_none() {
            break;
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TASKS;

    #[tokio::test]
    async fn honor_system_returns_true_without_calling_x() {
        let http = reqwest::Client::new();
        let task = &TASKS[0];
        assert!(verify_task(&http, task, "1", false, "").await);
        // Enabled, but this quest has no target id, so it stays on the honor system.
        assert!(verify_task(&http, task, "1", true, "token").await);
    }
}
