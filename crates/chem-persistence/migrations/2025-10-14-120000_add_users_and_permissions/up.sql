-- Migration: Add users, teams, and access control tables

CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(255) NOT NULL,
  email VARCHAR(255) UNIQUE NOT NULL,
  university VARCHAR(255),
  password_hash VARCHAR(255) NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE teams (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(255) NOT NULL,
  description TEXT,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE team_members (
  team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
  user_id UUID REFERENCES users(id) ON DELETE CASCADE,
  PRIMARY KEY (team_id, user_id)
);

CREATE TABLE molecule_family_access (
  family_id UUID REFERENCES molecule_families(id) ON DELETE CASCADE,
  accessor_id UUID NOT NULL,
  accessor_type VARCHAR(10) CHECK (accessor_type IN ('user', 'team')),
  PRIMARY KEY (family_id, accessor_id, accessor_type)
);

CREATE TABLE molecule_access (
  molecule_id UUID REFERENCES molecules(id) ON DELETE CASCADE,
  accessor_id UUID NOT NULL,
  accessor_type VARCHAR(10) CHECK (accessor_type IN ('user', 'team')),
  PRIMARY KEY (molecule_id, accessor_id, accessor_type)
);

CREATE TABLE flow_access (
  flow_id UUID REFERENCES flows(id) ON DELETE CASCADE,
  accessor_id UUID NOT NULL,
  accessor_type VARCHAR(10) CHECK (accessor_type IN ('user', 'team')),
  PRIMARY KEY (flow_id, accessor_id, accessor_type)
);

-- Indexes for performance
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_team_members_user_id ON team_members(user_id);
CREATE INDEX idx_molecule_family_access_accessor_id ON molecule_family_access(accessor_id);
CREATE INDEX idx_molecule_access_accessor_id ON molecule_access(accessor_id);
CREATE INDEX idx_flow_access_accessor_id ON flow_access(accessor_id);
