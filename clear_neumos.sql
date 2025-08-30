-- Clear all Neumos data from the database
-- Run with: sqlite3 seattle_music.db < clear_neumos.sql

-- Delete raw data for Neumos
DELETE FROM nodes WHERE label = 'raw_data' AND json_extract(data, '$.api_name') = 'crawler_neumos';

-- Delete events for Neumos
DELETE FROM nodes WHERE label = 'event' AND (
    json_extract(data, '$.venue_name') = 'Neumos' OR
    json_extract(data, '$.venue_name') = 'neumos' OR
    json_extract(data, '$.venue_slug') = 'neumos'
);

-- Delete venue for Neumos
DELETE FROM nodes WHERE label = 'venue' AND (
    json_extract(data, '$.name') = 'Neumos' OR
    json_extract(data, '$.name') = 'neumos' OR
    json_extract(data, '$.slug') = 'neumos'
);

-- Clean up orphaned edges
DELETE FROM edges WHERE source_id NOT IN (SELECT id FROM nodes) OR target_id NOT IN (SELECT id FROM nodes);

-- Show what's left
SELECT 'Raw data count:' as info, COUNT(*) as count FROM nodes WHERE label = 'raw_data' AND json_extract(data, '$.api_name') = 'crawler_neumos'
UNION ALL
SELECT 'Event count:', COUNT(*) FROM nodes WHERE label = 'event' AND json_extract(data, '$.venue_name') LIKE '%eumos%'
UNION ALL  
SELECT 'Venue count:', COUNT(*) FROM nodes WHERE label = 'venue' AND json_extract(data, '$.name') LIKE '%eumos%';
