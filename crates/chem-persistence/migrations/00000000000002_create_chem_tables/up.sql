CREATE TABLE IF NOT EXISTS molecules (
  inchikey TEXT PRIMARY KEY,
  smiles TEXT NOT NULL,
  inchi TEXT NOT NULL,
  metadata TEXT
);
CREATE TABLE IF NOT EXISTS families (
  id TEXT PRIMARY KEY,
  name TEXT,
  description TEXT,
  family_hash TEXT NOT NULL,
  provenance TEXT NOT NULL,
  frozen BOOLEAN NOT NULL DEFAULT TRUE
);
CREATE TABLE IF NOT EXISTS family_properties (
  id TEXT PRIMARY KEY,
  family_id TEXT NOT NULL,
  property_type TEXT NOT NULL,
  value TEXT NOT NULL,
  quality TEXT,
  preferred BOOLEAN NOT NULL DEFAULT FALSE,
  value_hash TEXT NOT NULL,
  metadata TEXT
);
CREATE TABLE IF NOT EXISTS molecular_properties (
  id TEXT PRIMARY KEY,
  molecule_inchikey TEXT NOT NULL,
  property_type TEXT NOT NULL,
  value TEXT NOT NULL,
  quality TEXT,
  preferred BOOLEAN NOT NULL DEFAULT FALSE,
  value_hash TEXT NOT NULL,
  metadata TEXT
);
CREATE TABLE IF NOT EXISTS family_members (
  id TEXT PRIMARY KEY,
  family_id TEXT NOT NULL,
  molecule_inchikey TEXT NOT NULL
);
