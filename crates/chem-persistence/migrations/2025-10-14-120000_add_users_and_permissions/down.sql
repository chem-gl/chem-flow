-- Rollback migration: Remove users, teams, and access control tables

DROP TABLE IF EXISTS flow_access;
DROP TABLE IF EXISTS molecule_access;
DROP TABLE IF EXISTS molecule_family_access;
DROP TABLE IF EXISTS team_members;
DROP TABLE IF EXISTS teams;
DROP TABLE IF EXISTS users;
