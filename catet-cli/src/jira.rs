/// Minimal Jira API client for the CLI.
/// Mirrors the subset of src-tauri/src/jira/client.rs that the CLI needs.
use base64::Engine;
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

const SEARCH_FIELDS: &str = "summary,status,project,parent";

#[derive(Debug, Clone)]
pub struct JiraClient {
    http: Client,
    base_url: String,
    auth_header: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorklogResult {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub issues: Vec<SearchIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIssue {
    pub key: String,
    pub fields: SearchIssueFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIssueFields {
    pub summary: String,
    #[serde(default)]
    pub status: Option<SearchNamedValue>,
    #[serde(default)]
    pub project: Option<SearchProject>,
    #[serde(default)]
    pub parent: Option<SearchParent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchNamedValue {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProject {
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParent {
    pub key: String,
    pub fields: SearchParentFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParentFields {
    pub summary: String,
}

impl JiraClient {
    pub fn new(domain: &str, email: &str, token: &str) -> Self {
        let base_url = if domain.starts_with("http") {
            domain.to_string()
        } else {
            format!("https://{}", domain)
        };
        let credentials = format!("{}:{}", email, token);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded);
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        Self { http, base_url, auth_header }
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, String> {
        Ok(self
            .http
            .request(method, format!("{}{}", self.base_url, path))
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json"))
    }

    fn map_error(status: reqwest::StatusCode, body: &str) -> String {
        match status.as_u16() {
            401 => "Authentication failed. Re-login in Catet Task.".to_string(),
            403 => "No permission to log work on this issue.".to_string(),
            404 => "Issue not found.".to_string(),
            429 => "Rate limited by Jira.".to_string(),
            _ if status.is_server_error() => format!("Jira server error ({}).", status),
            _ => format!("Jira API error ({}): {}", status, body),
        }
    }

    pub async fn search_issues_limited(
        &self,
        jql: &str,
        max_results: u32,
    ) -> Result<SearchResult, String> {
        let payload = json!({
            "jql": jql,
            "maxResults": max_results,
            "fields": SEARCH_FIELDS.split(',').collect::<Vec<&str>>(),
        });

        let req = self
            .request(reqwest::Method::POST, "/rest/api/3/search/jql")
            .await?;
        let resp = req
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_error(status, &body));
        }

        resp.json::<SearchResult>()
            .await
            .map_err(|e| format!("Failed to parse search results: {}", e))
    }

    pub async fn add_worklog(
        &self,
        issue_key: &str,
        time_spent_seconds: u64,
        started: &str,
        comment: &str,
    ) -> Result<WorklogResult, String> {
        let path = format!("/rest/api/3/issue/{}/worklog", issue_key);

        let formatted_started =
            NaiveDateTime::parse_from_str(started, "%Y-%m-%d %H:%M:%S")
                .map(|dt| {
                    let utc: DateTime<Utc> = dt.and_utc();
                    utc.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string()
                })
                .unwrap_or_else(|_| started.to_string());

        let comment_body = if comment.is_empty() {
            None
        } else {
            Some(json!({
                "type": "doc",
                "version": 1,
                "content": [{"type": "paragraph", "content": [{"type": "text", "text": comment}]}]
            }))
        };

        let payload = json!({
            "timeSpentSeconds": time_spent_seconds,
            "started": formatted_started,
            "comment": comment_body,
        });

        let mut retries = 0u32;
        loop {
            let resp = self
                .request(reqwest::Method::POST, &path)
                .await?
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("Network error: {}", e))?;

            if resp.status().is_success() {
                return resp
                    .json::<WorklogResult>()
                    .await
                    .map_err(|e| format!("Failed to parse worklog response: {}", e));
            }

            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();

            if (status.as_u16() == 429 || status.is_server_error()) && retries < 3 {
                retries += 1;
                tokio::time::sleep(Duration::from_secs(1 << retries)).await;
                continue;
            }

            return Err(Self::map_error(status, &body));
        }
    }
}
