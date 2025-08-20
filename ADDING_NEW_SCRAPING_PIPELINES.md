# Adding New Data Sources and Entity Types

A strategic guide for extending the data collection system with new sources and types of information.

## Understanding the Data Pipeline

This system operates like a sophisticated data factory that transforms raw information from various sources into clean, organized knowledge. Think of it as a series of quality control stations where each step has one focused responsibility, ensuring that every piece of information can trace its journey from source to final storage.

The information flows through these stages: **Sources → Ingestion → Parsing → Normalization → Quality Control → Enrichment → Entity Matching → Catalog Storage**. At each step, the system adds value while preserving the complete history of where each fact originated and how it was transformed.

## Current System Overview

The system currently manages three core types of information:
- **Events**: Individual performances, shows, or gatherings
- **Venues**: Physical locations where events happen  
- **Artists**: Performers who appear at events

There are currently three active data sources:
- **Blue Moon Tavern**: Collects data from a JSON-based web service
- **Sea Monster Lounge**: Extracts information from web pages that embed data in scripts
- **Darrell's Tavern**: Another venue data source (implementation exists)

Each data source follows the same pattern: it knows how to fetch raw information, identify individual records, extract the important details, and handle any source-specific quirks or filtering needs.

## Scenario 1: Adding a New Source for Existing Information Types

When you want to collect events from a new venue or platform, you're adding another voice to an existing conversation. The system already understands Events, Venues, and Artists—you just need to teach it how to listen to a new data source.

### Implementation Overview

#### 1. Create the Data Source Module

Every new data source needs its own dedicated module that acts as a translator between the external source and the internal system. This module lives in the APIs directory and follows a consistent naming pattern.

The module must implement four key capabilities:
- **Data Fetching**: How to retrieve raw information from the source
- **Record Identification**: How to recognize individual events within the raw data
- **Information Extraction**: How to pull out the important details (title, date, venue, etc.)
- **Quality Filtering**: How to skip records that shouldn't be processed

#### 2. Register System Identifiers

Each data source needs unique identifiers that the system uses for internal tracking and configuration. This involves:

- **Public Name**: The user-friendly identifier used in commands and interfaces
- **Internal Name**: The technical identifier used for data storage and logging
- **Display Name**: The human-readable name shown in reports and user interfaces

These identifiers must be added to the constants file and registered in the system's lookup functions so the platform knows about the new source.

#### 3. Register the Module

The new data source module must be registered with the system by adding it to the APIs module list. This tells the system that the new source exists and can be loaded when needed.

#### 4. Add Source Configuration

Each data source requires two types of configuration:

**Source Registry File**: A JSON file in the `registry/sources` directory that defines the source's identity, endpoints, rate limits, and data policies. This file acts as the official record of the data source and its expected behavior. The registry file must be named after your source (e.g., `kexp.json` for the KEXP source) and include these required fields:

```json
{
  "source_id": "kexp",
  "endpoint": "https://kexp.org/events/",
  "method": "GET",
  "rate_limit_requests_per_minute": 30,
  "rate_limit_requests_per_hour": 1000,
  "timeout_seconds": 30,
  "content_types": ["text/html"],
  "data_policy": "public",
  "description": "KEXP live music events and performances",
  "contact": "your-email@example.com",
  "last_updated": "2025-01-01"
}
```

This registry file is **required** for the ingester pipeline to recognize and process your new source. Without it, you'll encounter "No such file or directory" errors when attempting to run the pipeline.

**System Configuration**: Settings in the main `config.toml` file that control runtime behavior, such as timeouts, feature flags, and environment-specific settings.

#### 5. Implement Data Parser

Once you have a source that can fetch raw data, you need to implement a parser that can extract structured information from that raw data. The parser transforms the raw HTML, JSON, or XML into standardized records that the rest of the pipeline can process.

**Parser Implementation**: Each data source needs a parser that implements the `SourceParser` trait. This parser should:

- Handle the specific data format of your source (HTML selectors, JSON paths, XML parsing)
- Extract relevant fields like event title, date, time, location, and description
- Handle edge cases like missing fields, malformed data, or format variations
- Return structured `ParsedRecord` objects with consistent field naming

**Example Parser Structure**:
```rust
use scraper::{Html, Selector};
use crate::pipeline::processing::parser::{SourceParser, ParsedRecord};

pub struct YourSourceParser;

impl SourceParser for YourSourceParser {
    fn parse_html(&self, html: &str, source_id: &str, envelope_id: &str) -> Result<Vec<ParsedRecord>, Box<dyn std::error::Error>> {
        // Parse HTML and extract events
        // Return structured records
    }
    
    fn source_id(&self) -> &str {
        "your_source"
    }
}
```

**Parser Registration**: The parser must be registered in the parser factory (`src/pipeline/processing/parser/mod.rs`) so the system knows which parser to use for your source's `parse_plan_ref` value.

**Parser Testing**: Test your parser with real data from the source to ensure it handles various scenarios like different event types, missing optional fields, and date/time format variations.

#### 6. Implement Data Normalizer

After parsing extracts structured records, you need a normalizer to convert those records into the standard domain entities (Events, Venues, Artists) that the rest of the system expects.

**Normalizer Implementation**: Create a source-specific normalizer that implements the `SourceNormalizer` trait:

- Convert parsed records into domain entities (Event, Venue, Artist)
- Handle venue creation (usually one venue per source)
- Extract artist information from event titles when available
- Set appropriate confidence scores and normalization strategies
- Use state managers to prevent duplicate venue/artist creation

**Example Normalizer Structure**:
```rust
use crate::pipeline::processing::normalize::normalizers::base::{SourceNormalizer, NormalizerUtils, VenueStateManager, ArtistStateManager};
use crate::domain::{Artist, Event, Venue};

pub struct YourSourceNormalizer {
    venue_state: VenueStateManager,
    artist_state: ArtistStateManager,
}

impl SourceNormalizer for YourSourceNormalizer {
    fn normalize(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>> {
        // Convert parsed data to domain entities
        // Handle venue and artist creation
        // Return normalized records
    }
    
    fn source_id(&self) -> &str {
        "your_source"
    }
}
```

**Normalizer Registration**: Register your normalizer in the normalization registry (`src/pipeline/processing/normalize/registry.rs`) so it can be found by source ID.

#### 7. Testing and Validation

Before deploying a new data source, comprehensive testing ensures it works correctly:

- **Unit Testing**: Verify that data extraction functions handle various input formats correctly
- **Integration Testing**: Confirm that the source can successfully fetch and parse real data
- **Error Handling Testing**: Ensure graceful handling of missing fields, network issues, and malformed data
- **Edge Case Testing**: Test with unusual but valid data like very long titles, special characters, or extreme dates

Testing should cover both successful scenarios and failure modes to ensure the system remains stable even when external sources behave unexpectedly.

### Understanding the Data Journey

Once a new data source is implemented, here's what happens during processing:

1. **Raw Data Collection**: The system fetches information from the source and stores it permanently in its original form
2. **Data Parsing**: Your custom extraction logic converts raw data into structured records
3. **Normalization**: Standard rules ensure consistent formatting across all sources (addresses, dates, names)
4. **Quality Control**: Built-in validation checks data quality and flags or filters problematic records
5. **Information Enrichment**: The system adds geographic data, categorization tags, and contextual information
6. **Entity Matching**: New information is matched with existing venues, artists, and events to avoid duplicates
7. **Final Storage**: Clean, enriched data is stored in the searchable catalog with complete traceability

The key advantage is that you only implement the source-specific parsing—all the downstream processing, quality control, and storage happens automatically.

## Scenario 2: Adding New Types of Information

Sometimes you want to collect entirely new categories of data—perhaps food trucks, art installations, street performances, or community markets. This requires expanding the system's understanding of what kinds of entities exist and how they relate to each other.

### Implementation Strategy

#### 1. Define the New Information Model

Every new type of entity needs a clear definition of what information it contains and how that information is structured. Using food trucks as an example:

**Core Attributes**: Essential information like name, identifier, and category
**Descriptive Attributes**: Optional details like description, images, and external links
**Behavioral Attributes**: Information about how this entity behaves (for mobile entities, location tracking)
**Temporal Attributes**: When information was created or last updated
**Relationship Attributes**: How this entity connects to others (do food trucks appear at events?)

For mobile entities like food trucks, you might also need location history tracking to show where they've been and when.

#### 2. Create API Schema Definition

New entity types need to be exposed through the system's query interface (GraphQL API). This involves:

**Field Definitions**: What information can be requested about this entity
**Relationship Queries**: How to find related entities (current location for a food truck, events at a venue)
**Filtering Options**: Ways to search and filter the new entity type
**Mutation Operations**: Whether users can create, update, or delete these entities

The API schema acts as a contract, defining exactly what information is available and how it can be accessed by applications and users.

#### 3. Define Data Collection Interface

New entity types often require different data collection patterns than the existing event-focused approach. This might involve:

**Multiple Data Streams**: Food trucks might have both profile information and real-time location updates
**Different Update Frequencies**: Static information (menu, description) vs. dynamic information (current location)
**Specialized Extraction Methods**: Different ways to identify and parse the new entity type from raw data sources
**Custom Validation Rules**: Entity-specific logic for determining data quality and completeness

The data collection interface defines the contract that all sources of this entity type must implement, ensuring consistency across different data providers.

#### 4. Design Database Storage

New entity types require database schema changes to store the information effectively:

**Primary Tables**: Main storage for the new entity with all its core attributes
**Relationship Tables**: How the new entity connects to existing ones (many-to-many relationships)
**History Tables**: For entities that change over time (like location tracking)
**Index Strategy**: Database indexes optimized for common query patterns

**Performance Considerations**:
- Geographic queries for location-based entities
- Time-based queries for temporal data
- Text search for names and descriptions
- Foreign key relationships for entity connections

Database migrations ensure that schema changes can be applied safely in production environments without data loss.

#### 5. Implement Storage Operations

The storage layer needs to be extended to handle all operations for the new entity type:

**Create Operations**: Adding new entities to the database
**Read Operations**: Retrieving entities by ID, name, location, or other criteria
**Update Operations**: Modifying existing entity information
**Delete Operations**: Removing entities (if applicable)
**Relationship Operations**: Managing connections between entities
**Query Operations**: Complex searches and filtering

**Specialized Operations for Dynamic Entities**:
- Location tracking for mobile entities
- Historical data retrieval
- Real-time updates and notifications

#### 6. Build Processing Pipeline

New entity types require custom processing logic that handles their unique characteristics:

**Data Collection**: Gathering information from various sources with different update patterns
**Normalization**: Standardizing formats, cleaning data, and ensuring consistency
**Quality Control**: Validating entity-specific rules and requirements
**Enrichment**: Adding geographic data, categorization, or other contextual information
**Entity Matching**: Identifying when new information refers to existing entities
**Storage Integration**: Persisting processed data with proper relationships

**Special Considerations for Different Entity Types**:
- **Static Entities** (venues): Focus on accurate location and description data
- **Dynamic Entities** (food trucks): Handle real-time location updates and movement tracking
- **Event-Related Entities** (performances): Manage temporal data and artist relationships
- **Community Entities** (markets): Track recurring schedules and participant relationships

### Integration Strategy

When introducing new entity types, consider their relationship to the existing ecosystem:

**Entity Relationships**: How does the new type connect to existing ones? Food trucks might appear at events, art installations might be located at venues, street performances might feature known artists.

**Geographic Integration**: How does the new entity fit into location-based queries and mapping? Mobile entities need different spatial handling than fixed locations.

**Temporal Patterns**: Does the new entity follow event-like scheduling, or does it have different time-based behaviors? Some entities are permanent, others are temporary, some move regularly.

**User Interface Integration**: How will the new entity type appear in searches, lists, and detailed views? What filters and sorting options make sense?

**Data Quality Standards**: What validation rules ensure the new entity type maintains system-wide data quality standards?

## Implementation Guidelines and Best Practices

### Error Handling Strategy

Robust error handling is crucial for maintaining system stability when dealing with unpredictable external data sources:

**Graceful Degradation**: When data is missing or malformed, the system should continue processing other records rather than failing completely.

**Detailed Error Context**: Error messages should provide enough information for debugging, including the source name, record identifier, and specific field that caused the problem.

**Error Classification**: Different types of errors need different responses:
- **Missing Required Fields**: Skip the record but log the issue
- **Invalid Data Formats**: Attempt correction or skip with detailed logging
- **Network Issues**: Retry with exponential backoff
- **Authentication Problems**: Alert administrators immediately

**Recovery Mechanisms**: The system should be able to resume processing after errors without losing progress or duplicating work.

### Monitoring and Observability

Comprehensive monitoring ensures that data collection issues are detected and resolved quickly:

**Structured Logging**: Every operation should generate structured log entries that include:
- Source identifier and operation type
- Record counts and processing statistics
- Performance timing information
- Error details and context

**Metrics Collection**: Key performance indicators should be tracked:
- Records processed per source per day
- Success and failure rates
- Processing latency and throughput
- Data quality scores and validation failures

**Alerting Strategy**: Automated alerts for critical issues:
- Source unavailability or authentication failures
- Significant drops in data volume
- Increases in error rates
- Processing delays that affect freshness

**Dashboard Integration**: Visual monitoring that shows system health at a glance with historical trends and real-time status.

### Respectful Data Collection

Maintaining good relationships with data sources requires respectful collection practices:

**Rate Limiting**: Never overwhelm external sources with too many requests:
- Configure appropriate delays between requests based on the source's capacity
- Respect any rate limiting headers or guidance from the source
- Implement exponential backoff for retry attempts
- Consider the source's peak usage times and avoid adding load during busy periods

**Authentication and Authorization**: Properly handle API credentials:
- Use provided API keys or authentication tokens correctly
- Store credentials securely and rotate them as recommended
- Respect access level limitations and scope restrictions
- Monitor for authentication issues and respond appropriately

**Terms of Service Compliance**: Ensure collection practices align with the source's terms:
- Review and comply with data usage policies
- Respect robots.txt files for web scraping
- Cache appropriately to minimize redundant requests
- Honor opt-out requests and data removal requirements

### Data Quality Assurance

Maintaining high data quality requires validation at multiple levels:

**Field-Level Validation**: Each piece of extracted information should be validated:
- **Required Fields**: Ensure essential information is present and non-empty
- **Format Validation**: Verify dates, times, URLs, and other structured data are properly formatted
- **Range Validation**: Check that numeric values, dates, and coordinates fall within reasonable bounds
- **Content Validation**: Ensure text fields contain meaningful content, not placeholder text or error messages

**Record-Level Validation**: Evaluate the overall quality of each record:
- **Completeness**: Determine if enough information is present to create a useful record
- **Consistency**: Check that related fields make sense together (dates in logical order, coordinates matching addresses)
- **Uniqueness**: Identify and handle duplicate records from the same source

**Source-Level Validation**: Monitor the overall quality of data from each source:
- **Volume Validation**: Alert when sources provide significantly more or fewer records than expected
- **Pattern Recognition**: Identify when data formats change unexpectedly
- **Quality Trends**: Track quality metrics over time to detect degradation

### Testing Methodology

Thorough testing ensures that data sources work reliably across various scenarios:

**Unit Testing**: Test individual components in isolation:
- **Data Extraction**: Verify that parsing functions correctly handle various input formats
- **Validation Logic**: Ensure validation rules work correctly for both valid and invalid data
- **Error Handling**: Confirm that error conditions are handled gracefully
- **Edge Cases**: Test boundary conditions like empty responses, maximum field lengths, and unusual but valid data

**Integration Testing**: Test complete data collection workflows:
- **End-to-End Processing**: Verify that data flows correctly through all pipeline stages
- **Real Source Testing**: Test against actual data sources (with appropriate safeguards)
- **Performance Testing**: Ensure the system can handle expected data volumes efficiently
- **Error Recovery Testing**: Verify that the system recovers properly from various failure scenarios

**Data Quality Testing**: Validate the quality of collected information:
- **Sample Validation**: Manually review samples of collected data for accuracy
- **Historical Comparison**: Compare new implementations against previous versions
- **Cross-Source Validation**: When possible, verify data consistency across multiple sources
- **User Acceptance Testing**: Ensure collected data meets business requirements

## Common Challenges and Solutions

### Data Not Appearing in Final Results

When collected data doesn't show up in the system's query interface, the issue is usually in the processing pipeline:

**Data Collection Issues**: Verify that information is being successfully retrieved from the source
**Processing Failures**: Check if data is failing validation rules or quality gates
**Entity Matching Problems**: Confirm that venues, artists, and other entities are being matched or created correctly
**Storage Issues**: Ensure that processed data is being successfully saved to the database
**API Schema Problems**: Verify that the query interface is properly configured to return the new data

### Data Parsing Failures

When the system can't extract information from raw source data:

**Format Changes**: External sources may change their data format without notice—compare current responses to expected formats
**Field Availability**: Sources may add, remove, or rename fields—ensure extraction logic handles missing or renamed fields gracefully
**Data Type Changes**: Fields that were previously strings might become numbers or arrays—implement flexible type handling
**Content Variations**: Real-world data often contains unexpected variations—test with actual source data, not just ideal examples

### Source Access Problems

When external sources become difficult to access:

**Rate Limiting**: Sources may limit how frequently you can request data—adjust request timing and implement proper backoff strategies
**Authentication Issues**: API keys may expire or change—monitor authentication status and implement renewal procedures
**Source Unavailability**: External sources may experience downtime—implement retry logic with appropriate delays
**Access Restrictions**: Sources may block automated access—review terms of service and consider alternative access methods

### Data Quality Problems

When collected information doesn't meet quality standards:

**Incomplete Records**: Sources may provide partial information—determine acceptable minimum requirements and handle missing fields appropriately
**Invalid Data**: Information may be malformed or nonsensical—implement validation rules and data cleaning procedures
**Inconsistent Formats**: Different sources may format the same information differently—develop normalization strategies for common fields like dates and addresses
**Duplicate Information**: Sources may provide redundant data—implement deduplication logic that works both within and across sources

## Advanced Implementation Strategies

### Custom Quality Control

Different entity types require specialized quality validation:

**Entity-Specific Rules**: Each type of information has unique validation requirements—events need valid dates and times, venues need proper addresses and coordinates, mobile entities need location tracking capabilities.

**Contextual Validation**: Quality rules should consider the context—a food truck without a current location might be acceptable if it's temporarily closed, but an event without a date is always problematic.

**Severity Levels**: Not all quality issues are equal—missing optional information might generate warnings, while invalid required fields should cause rejection.

**Adaptive Rules**: Quality standards may need to adjust based on the source reliability and the intended use of the data.

### Advanced Entity Matching

Different types of entities require specialized strategies for identifying when new information refers to existing records:

**Exact Matching**: Some entities can be matched precisely using unique identifiers, official names, or addresses.

**Fuzzy Matching**: When exact matches aren't possible, similarity algorithms can identify likely matches based on name variations, alternative spellings, or partial information.

**Multi-Factor Matching**: Complex entities might require matching on multiple attributes—a venue might be matched by both name and address, while an artist might be matched by name and associated venues.

**Temporal Considerations**: Some entities change over time—restaurants change names, food trucks change locations, artists change performance styles. The matching logic needs to account for these temporal aspects.

**Confidence Scoring**: When multiple potential matches exist, confidence scores help determine the most likely match or whether to create a new entity instead.

## Strategic Summary

Expanding this data collection system is like growing a living knowledge network—each new source adds another perspective, while each new entity type broadens the system's understanding of the world. Success comes from respecting the core architectural principles: preserve original information, maintain complete traceability, keep processing steps simple and predictable, and ensure each component has a focused responsibility.

### Key Success Factors

**Respect the Data Flow**: The system works best when each stage focuses on its specific role without trying to do everything at once.

**Preserve Traceability**: Every piece of information should be able to explain its journey from source to final storage.

**Plan for Change**: External sources evolve constantly—build flexibility into your extraction and validation logic.

**Monitor Continuously**: Data collection systems require ongoing attention to maintain quality and reliability.

**Test Thoroughly**: Comprehensive testing prevents small issues from becoming large problems in production.

### Long-term Considerations

As the system grows, consider the broader ecosystem: How do new entities relate to existing ones? What new query patterns do users need? How can data quality be maintained as volume increases? What opportunities exist for cross-source validation and enrichment?

The system's architecture makes it straightforward to add new capabilities while maintaining the clarity and reliability that make it valuable. Whether you're adding a single new venue or introducing an entirely new category of information, the patterns and principles remain consistent.

Remember: predictable inputs produce predictable outputs, original data is irreplaceable, every fact should be traceable to its source, and comprehensive logging makes debugging possible when things go wrong.
