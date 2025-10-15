-- Migration: Add users, teams, and access control tables (SQLite/Postgres portable)

-- Use TEXT for UUID columns for portability across SQLite and Postgres. The application
-- inserts and parses UUID strings.

CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  email TEXT UNIQUE NOT NULL,
  university TEXT,
  password_hash TEXT NOT NULL,
  created_at INTEGER,
  updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS teams (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  created_at INTEGER,
  updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS team_members (
  team_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  PRIMARY KEY (team_id, user_id)
  -- FK constraints removed for portability across backends in tests
  -- FOREIGN KEY(team_id) REFERENCES teams(id) ON DELETE CASCADE,
  -- FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS molecule_family_access (
  family_id TEXT NOT NULL,
  accessor_id TEXT NOT NULL,
  accessor_type TEXT CHECK (accessor_type IN ('user','team')),
  PRIMARY KEY (family_id, accessor_id, accessor_type)
  -- FK removed: original reference pointed to molecule_families, which doesn't exist
  -- FOREIGN KEY(family_id) REFERENCES families(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS molecule_access (
  molecule_id TEXT NOT NULL,
  accessor_id TEXT NOT NULL,
  accessor_type TEXT CHECK (accessor_type IN ('user','team')),
  PRIMARY KEY (molecule_id, accessor_id, accessor_type)
  -- FK removed because molecules table uses inchikey as primary key in current schema
  -- FOREIGN KEY(molecule_id) REFERENCES molecules(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS flow_access (
  flow_id TEXT NOT NULL,
  accessor_id TEXT NOT NULL,
  accessor_type TEXT CHECK (accessor_type IN ('user','team')),
  PRIMARY KEY (flow_id, accessor_id, accessor_type),
  FOREIGN KEY(flow_id) REFERENCES flows(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_team_members_user_id ON team_members(user_id);
CREATE INDEX IF NOT EXISTS idx_molecule_family_access_accessor_id ON molecule_family_access(accessor_id);
CREATE INDEX IF NOT EXISTS idx_molecule_access_accessor_id ON molecule_access(accessor_id);
CREATE INDEX IF NOT EXISTS idx_flow_access_accessor_id ON flow_access(accessor_id);
