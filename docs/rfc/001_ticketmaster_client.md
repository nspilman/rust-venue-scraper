## RFC-001 (Revised): Ticketmaster API Client Implementation

**Author:** Gemini (Senior Rust Engineer)
**Date:** 2025-07-30 
**Status:** Approved

### 1. Summary

This RFC proposes the design for the first concrete implementation of the `EventApi` trait: a client for the Ticketmaster Discovery API. This component will be responsible for fetching music events in the Seattle area. Its successful implementation will validate our core `EventApi` trait, establish patterns for future API clients, and provide the first real data for our scraper pipeline.

### 2. Motivation

To iteratively build our Rust-based scraper, we must begin implementing data sources. The Ticketmaster API is an ideal starting point because:
- It is a well-structured REST API with predictable JSON responses.
- It provides rich data, including venue, artist (implicitly), and event details, which will exercise all the methods on our `EventApi` trait.
- The existing Python implementation provides a clear reference for required logic, such as pagination and rate limiting.
- It serves as a low-risk, high-reward first implementation to prove out our foundational architecture.

### 3. Detailed Design

#### 3.1. File Structure

The implementation will reside in a new module:

```
src/
├── apis/
│   └── ticketmaster.rs
├── main.rs
└── types.rs
```

We will create `src/apis/mod.rs` to declare the `ticketmaster` module.

#### 3.2. Data Structures (DTOs)

We will define Rust structs that mirror the JSON structure of the Ticketmaster API response. These will be our Data Transfer Objects (DTOs), primarily used for deserialization with `serde`.

```rust
// In src/apis/ticketmaster.rs

use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct TicketmasterResponse {
    #[serde(rename = "_embedded")]
    embedded: Option<TicketmasterEmbedded>,
    page: TicketmasterPageInfo,
}

#[derive(Deserialize, Debug)]
struct TicketmasterEmbedded {
    events: Vec<TicketmasterEvent>,
}

#[derive(Deserialize, Debug)]
struct TicketmasterEvent {
    id: String,
    name: String,
    url: String,
    images: Vec<TicketmasterImage>,
    dates: TicketmasterDates,
    #[serde(rename = "_embedded")]
    embedded: Option<EventEmbedded>,
}

#[derive(Deserialize, Debug)]
struct EventEmbedded {
    venues: Vec<TicketmasterVenue>,
    // attractions are often artists, but the data is less reliable
}

#[derive(Deserialize, Debug)]
struct TicketmasterVenue {
    id: String,
    name: String,
    location: Option<TicketmasterLocation>,
    // ... other relevant venue fields
}

// ... other necessary structs for Dates, Images, Location, etc.
```

#### 3.3. The `TicketmasterApi` Struct

This struct will hold the state required to interact with the Ticketmaster API.

```rust
// In src/apis/ticketmaster.rs

pub struct TicketmasterApi {
    client: reqwest::Client,
    api_key: String,
}

impl TicketmasterApi {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }
}
```

#### 3.4. `EventApi` Trait Implementation

We will implement the `EventApi` trait for `TicketmasterApi`.

- `api_name()`: Will return the static string `"ticketmaster"`.
- `get_event_list()`: This is the core method.
    1. It will construct the initial URL for the Ticketmaster Discovery API, including the `geoPoint` for Seattle and the API key.
    2. It will loop, fetching one page of results at a time.
    3. After the first request, it will read the `totalPages` from the response's page info to control the loop.
    4. To respect the API's rate limit (5 requests/second), it will introduce a `tokio::time::sleep` delay between each paginated request, as was done in the Python version.
    5. It will collect all `TicketmasterEvent` objects from all pages into a single `Vec`.
- `get_raw_data_info()`, `get_venue_args()`, `get_event_args()`: These methods will be responsible for mapping the data from the deserialized `TicketmasterEvent` DTO to our application's generic `RawDataInfo`, `VenueArgs`, and `EventArgs` structs. This creates a clean boundary between API-specific data and our internal models. The logic for selecting the best image URL from the `images` array will be re-implemented here.

#### 3.5. Error Handling

All public-facing methods will return `anyhow::Result<T>` (or a similar project-wide `Result` type). This will allow us to use the `?` operator to propagate errors from `reqwest` (network errors), `serde_json` (deserialization errors), and our own business logic (e.g., missing required fields in the response).

#### 3.6. Configuration & Secrets

The Ticketmaster API key is a secret and must not be hardcoded. I will use the `dotenv` crate to load the key from a `.env` file in the project root. The `main` function will be responsible for reading this key and passing it to the `TicketmasterApi::new()` constructor.

### 4. Implementation Steps

1. **Create Module:** Create the `src/apis/mod.rs` and `src/apis/ticketmaster.rs` files.
2. **Define DTOs:** Add the `serde`-based DTO structs to `ticketmaster.rs`.
3. **Implement Struct:** Create the `TicketmasterApi` struct and its `new` function.
4. **Implement `get_event_list()`:** Write the asynchronous logic for fetching events, including pagination and rate-limiting delays.
5. **Implement Mapping Methods:** Write the data mapping logic for the `get_*_args()` methods.
6. **Handle Secrets:** Integrate `dotenv` to load the `TICKETMASTER_API_KEY` from a `.env` file.
7. **Integrate with `main.rs`:** Update `main.rs` to instantiate `TicketmasterApi` and call `get_event_list()` when the `ingester` command is run with the `ticketmaster` API specified. For now, it will just print the results to the console.

### 5. Updated Design - Responses to Open Questions

1. **API Key:** ✅ Will be added to `.env` file
2. **Rate Limiting:** ✅ Will be configurable via TOML config file  
3. **Geographic Area:** ✅ Seattle will be configurable via TOML config file
4. **Error Handling:** ✅ Will implement Rust best practices using `thiserror` for structured error types

#### 5.1. Configuration System

Since we need configuration for multiple settings, we'll create a proper config system:

```rust
// src/config.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub ticketmaster: TicketmasterConfig,
}

#[derive(Debug, Deserialize)]  
pub struct TicketmasterConfig {
    pub delay_ms: u64,
    pub geo_point: String,
    pub timeout_seconds: u64,
}
```

With a corresponding `config.toml`:
```toml
[ticketmaster]
delay_ms = 500
geo_point = "c22zp"  # Seattle
timeout_seconds = 15
```

#### 5.2. Error Handling (Rust Best Practices)

For Rust best practices, we'll use `thiserror` to create structured error types:

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScraperError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("JSON deserialization failed: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("API error: {message}")]
    Api { message: String },
}

pub type Result<T> = std::result::Result<T, ScraperError>;
```

This approach:
- Uses `thiserror` for clean error definitions
- Provides automatic `From` implementations for common error types
- Creates a project-wide `Result<T>` type alias
- Follows Rust conventions for library error handling

