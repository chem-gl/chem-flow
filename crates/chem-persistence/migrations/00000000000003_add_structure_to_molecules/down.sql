-- NOTE: SQLite does not support DROP COLUMN directly. To revert this migration,
-- a full table recreate would be required. For now this down migration is a
-- no-op for SQLite. If you use Postgres, replace with: ALTER TABLE molecules DROP COLUMN structure;

-- no-op for sqlite
