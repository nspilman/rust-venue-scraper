-- Add fields to track normalization and quality gate status
-- This migration adds fields to the nodes table to track the status of normalization and quality gate processing

-- Add status fields to nodes table
ALTER TABLE nodes ADD COLUMN normalized_at TEXT;
ALTER TABLE nodes ADD COLUMN normalization_errors TEXT;
ALTER TABLE nodes ADD COLUMN quality_checked_at TEXT;
ALTER TABLE nodes ADD COLUMN quality_score REAL;
ALTER TABLE nodes ADD COLUMN quality_decision TEXT; -- 'accept', 'accept_with_warnings', 'quarantine'
ALTER TABLE nodes ADD COLUMN quality_errors TEXT;

-- Create an index for querying unnormalized nodes
CREATE INDEX IF NOT EXISTS idx_nodes_unnormalized 
  ON nodes(label) 
  WHERE normalized_at IS NULL AND label = 'raw_data';

-- Create an index for querying nodes by quality decision
CREATE INDEX IF NOT EXISTS idx_nodes_quality_decision 
  ON nodes(quality_decision) 
  WHERE label = 'raw_data';

-- Create a view for unprocessed raw data
CREATE VIEW IF NOT EXISTS vw_unprocessed_raw_data AS
SELECT * FROM nodes 
WHERE label = 'raw_data' 
  AND normalized_at IS NULL;

-- Create a view for normalized but unprocessed data
CREATE VIEW IF NOT EXISTS vw_unnormalized_raw_data AS
SELECT * FROM nodes 
WHERE label = 'raw_data' 
  AND normalized_at IS NULL;

-- Create a view for data that needs quality check
CREATE VIEW IF NOT EXISTS vw_unchecked_quality_data AS
SELECT * FROM nodes 
WHERE label = 'raw_data' 
  AND normalized_at IS NOT NULL 
  AND quality_checked_at IS NULL;

-- Create a view for quarantined data
CREATE VIEW IF NOT EXISTS vw_quarantined_data AS
SELECT * FROM nodes 
WHERE label = 'raw_data' 
  AND quality_decision = 'quarantine';

-- Update the trigger to handle new fields
DROP TRIGGER IF EXISTS nodes_updated_at;
CREATE TRIGGER nodes_updated_at 
  AFTER UPDATE ON nodes
BEGIN
  UPDATE nodes 
  SET updated_at = datetime('now') 
  WHERE id = NEW.id;
END;
