CREATE TABLE IF NOT EXISTS flows (
  id TEXT PRIMARY KEY,
  name TEXT,
  status TEXT,
  created_by TEXT,
  created_at_ts BIGINT,
  current_cursor BIGINT,
  current_version BIGINT,
  parent_flow_id TEXT,
  parent_cursor BIGINT,
  metadata TEXT
);
CREATE TABLE IF NOT EXISTS flow_data (
  id TEXT PRIMARY KEY,
  flow_id TEXT NOT NULL,
  cursor BIGINT,
  key TEXT,
  payload TEXT,
  metadata TEXT,
  command_id TEXT,
  created_at_ts BIGINT
);
CREATE TABLE IF NOT EXISTS snapshots (
  id TEXT PRIMARY KEY,
  flow_id TEXT NOT NULL,
  cursor BIGINT,
  state_ptr TEXT,
  metadata TEXT,
  created_at_ts BIGINT
);
