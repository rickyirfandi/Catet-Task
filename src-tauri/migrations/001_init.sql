CREATE TABLE IF NOT EXISTS users (
  id            TEXT PRIMARY KEY,
  email         TEXT NOT NULL,
  display_name  TEXT,
  avatar_url    TEXT,
  jira_domain   TEXT NOT NULL,
  auth_method   TEXT DEFAULT 'api_token',
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS tasks (
  id            TEXT PRIMARY KEY,
  summary       TEXT NOT NULL,
  project_key   TEXT,
  project_name  TEXT,
  status        TEXT,
  sprint_name   TEXT,
  pinned        BOOLEAN DEFAULT 0,
  last_fetched  DATETIME,
  created_at    DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS time_entries (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id         TEXT NOT NULL REFERENCES tasks(id),
  start_time      DATETIME NOT NULL,
  end_time        DATETIME,
  duration_secs   INTEGER,
  adjusted_secs   INTEGER,
  description     TEXT,
  synced_to_jira  BOOLEAN DEFAULT 0,
  jira_worklog_id TEXT,
  created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS settings (
  key    TEXT PRIMARY KEY,
  value  TEXT
);

CREATE TABLE IF NOT EXISTS schema_versions (
  version     INTEGER PRIMARY KEY,
  name        TEXT NOT NULL,
  applied_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_entries_task ON time_entries(task_id);
CREATE INDEX IF NOT EXISTS idx_entries_date ON time_entries(start_time);
CREATE INDEX IF NOT EXISTS idx_entries_synced ON time_entries(synced_to_jira);
