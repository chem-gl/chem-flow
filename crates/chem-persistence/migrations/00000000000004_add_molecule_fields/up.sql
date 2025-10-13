ALTER TABLE molecules ADD COLUMN id TEXT;
ALTER TABLE molecules ADD COLUMN created_at_ts BIGINT;
ALTER TABLE molecules ADD COLUMN updated_at_ts BIGINT;
ALTER TABLE molecules ADD COLUMN version INTEGER DEFAULT 1;
ALTER TABLE molecules ADD COLUMN molecular_formula TEXT;