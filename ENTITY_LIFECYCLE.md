# Entity Lifecycle Documentation

## Overview

This document describes the complete lifecycle of entities (Venues, Events, Artists) as they flow through the data pipeline from raw API responses to persisted catalog entries. Each stage has a specific purpose and data structure.

## Pipeline Stages and Data Objects

### Stage 1: RAW INGESTION
**Object: `RawPayload`**
- **Location**: Gateway/Fetch stage
- **Purpose**: Unprocessed API response data
- **Structure**: JSON blob, API-specific format
- **Example**: Raw JSON from Ticketmaster, Bandsintown, etc.
- **Key Operations**: 
  - Fetch from external API
  - Store in content-addressed storage
  - Generate envelope with metadata

### Stage 2: PARSING
**Object: `ParsedRecord`**
- **Location**: Parser stage
- **Purpose**: Structured extraction from raw payload
- **Structure**: 
  ```rust
  ParsedRecord {
      record: Map<String, Value>,  // Key-value pairs extracted from raw
      source_metadata: SourceInfo, // Where this came from
      parser_confidence: f64,       // How confident the parser is
  }
  ```
- **Key Operations**:
  - Extract fields from raw JSON
  - Handle API-specific formats
  - Validate required fields exist

### Stage 3: NORMALIZATION
**Object: `NormalizedEntity`**
- **Location**: Normalizer stage
- **Purpose**: Convert to canonical domain model
- **Structure**: 
  ```rust
  enum NormalizedEntity {
      Venue(NormalizedVenue),   // Standard venue representation
      Event(NormalizedEvent),   // Standard event representation
      Artist(NormalizedArtist), // Standard artist representation
  }
  ```
- **Key Fields**:
  - NormalizedVenue: name, address, coordinates, capacity
  - NormalizedEvent: title, date, time, venue_ref, artist_refs
  - NormalizedArtist: name, genre, bio
- **Key Operations**:
  - Map parsed fields to domain model
  - Geocode addresses to coordinates
  - Normalize dates/times to standard format
  - Generate temporary IDs

### Stage 4: QUALITY ASSESSMENT
**Object: `QualityAssessedRecord`**
- **Location**: Quality Gate stage
- **Purpose**: Validate and score data quality
- **Structure**:
  ```rust
  QualityAssessedRecord {
      normalized_record: NormalizedRecord,
      quality_score: QualityScore,
      validation_results: Vec<ValidationResult>,
      issues: Vec<QualityIssue>,
  }
  ```
- **Key Operations**:
  - Validate required fields
  - Check data consistency
  - Score completeness
  - Flag quality issues

### Stage 5: ENRICHMENT
**Object: `EnrichedEntity`**
- **Location**: Enrichment stage
- **Purpose**: Add derived and external data
- **Structure**:
  ```rust
  EnrichedEntity {
      base_entity: NormalizedEntity,
      enrichments: {
          geocoded_location: Option<GeoPoint>,
          venue_capacity: Option<i32>,
          artist_popularity: Option<f64>,
          genre_tags: Vec<String>,
          related_entities: Vec<EntityRef>,
      }
  }
  ```
- **Key Operations**:
  - Geocode addresses
  - Lookup venue details
  - Fetch artist metadata
  - Add classification tags

### Stage 6: CONFLATION
**Object: `ConflatedEntity`**
- **Location**: Conflation stage
- **Purpose**: Entity resolution and deduplication
- **Structure**:
  ```rust
  ConflatedEntity {
      canonical_id: EntityId,      // Stable, persistent ID
      enriched_entity: EnrichedEntity,
      resolution: ResolutionDecision,
      alternatives: Vec<AlternativeMatch>,
      confidence: f64,
  }
  ```
- **Resolution Decisions**:
  - `NewEntity` - First time seeing this entity
  - `MatchedExisting(id)` - Matches existing entity exactly
  - `UpdatedExisting(id)` - Updates existing with new info
  - `Duplicate(id)` - Duplicate with no new information
- **Key Operations**:
  - Match against existing entities
  - Resolve duplicates
  - Assign canonical IDs
  - Track lineage

### Stage 7: CATALOG PREPARATION
**Object: `CatalogCandidate`**
- **Location**: Catalog stage (pre-persistence)
- **Purpose**: Prepare for storage with change detection
- **Structure**:
  ```rust
  CatalogCandidate {
      entity_type: EntityType,
      canonical_id: EntityId,
      proposed_state: ProposedEntity,
      current_state: Option<PersistedEntity>,
      changes: ChangeSet,
      should_persist: bool,
  }
  ```
- **Change Detection**:
  - No existing entity → Create
  - Existing with changes → Update
  - Existing with no changes → Skip
- **Key Operations**:
  - Load current state from storage
  - Detect changes
  - Prepare update/insert operations

### Stage 8: PERSISTENCE
**Object: `PersistedEntity`**
- **Location**: Storage layer
- **Purpose**: Final stored representation
- **Structure**:
  ```rust
  // These match your domain models
  PersistedVenue {
      id: Uuid,
      name: String,
      address: String,
      city: String,
      // ... other fields
      created_at: DateTime,
      updated_at: DateTime,
  }
  ```
- **Key Operations**:
  - Insert new entities
  - Update existing entities
  - Maintain audit trail
  - Generate ProcessRecords

### Stage 9: PROVENANCE
**Object: `ProcessRecord`**
- **Location**: Audit/Lineage tracking
- **Purpose**: Track all changes and operations
- **Structure**:
  ```rust
  ProcessRecord {
      id: Uuid,
      process_run_id: Uuid,
      api_name: String,
      change_type: String,    // CREATE, UPDATE, NO_CHANGE
      change_log: String,      // Human readable description
      field_changed: String,   // Which fields changed
      entity_id: Uuid,         // What entity was affected
      created_at: DateTime,
  }
  ```

## Data Flow Summary

```
RawPayload 
    ↓ (parse)
ParsedRecord
    ↓ (normalize) 
NormalizedEntity
    ↓ (assess quality)
QualityAssessedRecord
    ↓ (enrich)
EnrichedEntity
    ↓ (conflate/resolve)
ConflatedEntity
    ↓ (prepare for catalog)
CatalogCandidate
    ↓ (persist if needed)
PersistedEntity
    ↓ (track lineage)
ProcessRecord
```

## Handler Responsibilities

Each EntityHandler in the registry pattern should own the transformation through these stages:

1. **Extract**: ConflatedEntity → CatalogCandidate
2. **Compare**: CatalogCandidate ↔ PersistedEntity 
3. **Persist**: CatalogCandidate → PersistedEntity
4. **Track**: All operations → ProcessRecord

## Key Principles

1. **Immutability**: Each stage produces a new object; never mutate in place
2. **Traceability**: Every transformation is tracked with metadata
3. **Idempotency**: Running the same data twice produces the same result
4. **Separation**: Each stage has a single responsibility
5. **Testability**: Each transformation can be tested in isolation

## Implementation Notes

### Current State
Your codebase already has most of these stages implemented:
- ParsedRecord ✓
- NormalizedEntity (as NormalizedRecord containing entity) ✓
- QualityAssessedRecord ✓
- EnrichedRecord ✓
- ConflatedRecord ✓
- PersistedEntity (Venue, Event, Artist in domain) ✓
- ProcessRecord ✓

### What's Missing
- **CatalogCandidate**: This intermediate stage between conflation and persistence
- **Clear handler ownership**: Handlers should own the full transformation pipeline
- **Consistent naming**: Some objects have unclear names (e.g., NormalizedRecord vs NormalizedEntity)

### Recommended Refactoring

1. **Rename for clarity**:
   - `NormalizedRecord` → `NormalizedEntity` (consistently)
   - Make the enum variants clear: `NormalizedVenue`, `NormalizedEvent`, `NormalizedArtist`

2. **Add CatalogCandidate stage**:
   - Bridges between conflation and persistence
   - Owns change detection logic
   - Decides whether to persist

3. **Refactor handlers** to own:
   - Extraction from ConflatedEntity
   - Creation of CatalogCandidate
   - Comparison with PersistedEntity
   - Generation of ProcessRecords

This architecture ensures each stage has a clear purpose and the data transformation is traceable and testable throughout the pipeline.
