-- Add migration script here
ALTER TABLE coffees DROP COLUMN name;
ALTER TABLE coffees ADD COLUMN roastery TEXT NOT NULL;