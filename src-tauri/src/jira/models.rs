use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraUser {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "emailAddress", default)]
    pub email_address: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "avatarUrls", default)]
    pub avatar_urls: AvatarUrls,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AvatarUrls {
    #[serde(rename = "48x48", default)]
    pub large: String,
    #[serde(rename = "32x32", default)]
    pub medium: String,
    #[serde(rename = "24x24", default)]
    pub small: String,
    #[serde(rename = "16x16", default)]
    pub xsmall: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraSearchResult {
    #[serde(default)]
    pub issues: Vec<JiraIssue>,
    #[serde(default)]
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub fields: JiraIssueFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssueFields {
    pub summary: String,
    #[serde(default)]
    pub status: Option<JiraStatus>,
    #[serde(default)]
    pub project: Option<JiraProject>,
    #[serde(default)]
    pub description: Option<serde_json::Value>,
    #[serde(rename = "issuetype", default)]
    pub issue_type: Option<JiraIssueType>,
    #[serde(default)]
    pub priority: Option<JiraPriority>,
    #[serde(default)]
    pub assignee: Option<JiraAssignee>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraStatus {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraProject {
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssueType {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraPriority {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraAssignee {
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorklogPayload {
    pub time_spent_seconds: u64,
    pub started: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraWorklog {
    pub id: String,
}

/// Frontend-facing user struct
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUser {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: String,
    pub jira_domain: String,
    pub auth_method: String,
}

/// Frontend-facing task struct
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTask {
    pub id: String,
    pub summary: String,
    pub project_key: String,
    pub project_name: String,
    pub status: String,
    pub sprint_name: Option<String>,
    pub pinned: bool,
    pub last_fetched: Option<String>,
}

/// Frontend-facing task detail struct
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTaskDetail {
    pub task_id: String,
    pub summary: String,
    pub description: Option<String>,
    pub status: String,
    pub project_key: String,
    pub project_name: String,
    pub issue_type: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
}

/// Frontend-facing time entry struct
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTimeEntry {
    pub id: i64,
    pub task_id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration_secs: Option<i64>,
    pub adjusted_secs: Option<i64>,
    pub description: Option<String>,
    pub synced_to_jira: bool,
    pub jira_worklog_id: Option<String>,
}

/// Frontend-facing timer state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTimerState {
    pub status: String,
    pub task_id: Option<String>,
    pub elapsed_secs: u64,
}
