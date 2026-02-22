use base64::Engine;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

use super::models::*;

const SEARCH_FIELDS: &str = "summary,status,project";

#[derive(Debug, Clone)]
pub struct JiraClient {
    http: Client,
    pub base_url: String,
    auth_header: String,
    pub email: String,
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

        Self {
            http,
            base_url,
            auth_header,
            email: email.to_string(),
        }
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
            401 => "Authentication failed. Please check your credentials and try again.".to_string(),
            403 => "You don't have permission to access this resource.".to_string(),
            404 => "Resource not found.".to_string(),
            429 => "Rate limited by Jira. Please wait a moment and try again.".to_string(),
            _ if status.is_server_error() => {
                format!("Jira server error ({}). Please try again later.", status)
            }
            _ => format!("Jira API error ({}): {}", status, body),
        }
    }

    pub async fn verify_auth(&self) -> Result<JiraUser, String> {
        let req = self
            .request(reqwest::Method::GET, "/rest/api/3/myself")
            .await?;
        let resp = req.send().await.map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_error(status, &body));
        }

        resp.json::<JiraUser>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    pub async fn search_issues(&self, jql: &str) -> Result<JiraSearchResult, String> {
        let body = json!({
            "jql": jql,
            "maxResults": 50,
            "fields": SEARCH_FIELDS.split(',').collect::<Vec<&str>>()
        });

        let req = self.request(reqwest::Method::POST, "/rest/api/3/search/jql").await?;
        let resp = req.json(&body).send().await.map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_error(status, &body));
        }

        resp.json::<JiraSearchResult>()
            .await
            .map_err(|e| format!("Failed to parse search results: {}", e))
    }

    pub async fn get_issue(&self, key: &str) -> Result<JiraIssue, String> {
        let path = format!("/rest/api/3/issue/{}?fields={}", key, SEARCH_FIELDS);
        let req = self.request(reqwest::Method::GET, &path).await?;
        let resp = req.send().await.map_err(|e| format!("Network error: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_error(status, &body));
        }

        resp.json::<JiraIssue>()
            .await
            .map_err(|e| format!("Failed to parse issue: {}", e))
    }

    pub async fn add_worklog(
        &self,
        issue_key: &str,
        time_spent_seconds: u64,
        started: &str,
        comment: &str,
    ) -> Result<JiraWorklog, String> {
        let path = format!("/rest/api/3/issue/{}/worklog", issue_key);

        let comment_body = if comment.is_empty() {
            None
        } else {
            Some(json!({
                "type": "doc",
                "version": 1,
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "text",
                        "text": comment
                    }]
                }]
            }))
        };

        let payload = WorklogPayload {
            time_spent_seconds,
            started: started.to_string(),
            comment: comment_body,
        };

        let mut retries = 0;
        loop {
            let req = self.request(reqwest::Method::POST, &path).await?;
            let resp = req
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("Network error: {}", e))?;

            if resp.status().is_success() {
                return resp
                    .json::<JiraWorklog>()
                    .await
                    .map_err(|e| format!("Failed to parse worklog response: {}", e));
            }

            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();

            // Retry on 429 or 5xx
            if (status.as_u16() == 429 || status.is_server_error()) && retries < 3 {
                retries += 1;
                let backoff = Duration::from_secs(1 << retries); // 2s, 4s, 8s
                tokio::time::sleep(backoff).await;
                continue;
            }

            return Err(Self::map_error(status, &body));
        }
    }
}

