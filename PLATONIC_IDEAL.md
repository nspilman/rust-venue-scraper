# Platonic Ideal: Calm, Explainable End‑to‑End Data Flow

A human‑readable description of the intended architecture and operating principles.

> Original note: “heres the platonic ideal for this project in human readable explainer form. Please study this ideal and save the content to a local markdwon file for future context.”

## At‑a‑Glance
- Sources → Ingestion Gateway → Ingest Log → Parse → Normalize → Quality Gate → Enrich → Conflation → Catalog
- Immutable raw bytes; everything else is append‑only history
- Deterministic, replayable steps with provenance
- Loosely coupled edges; fairness and policy at the door

## Table of Contents
- [1. Sources and Registry](#1-sources-and-registry)
- [2. Ingestion Gateway](#2-ingestion-gateway)
- [3. Ingest Log](#3-ingest-log)
- [4. Parse](#4-parse)
- [5. Normalize](#5-normalize)
- [6. Quality Gate](#6-quality-gate)
- [7. Enrich](#7-enrich)
- [8. Conflation](#8-conflation)
- [9. Catalog](#9-catalog)
- [10. Operational Patterns and Storage](#10-operational-patterns-and-storage)
- [11. Core Principles](#11-core-principles)
- [12. End‑to‑End Flow Summary](#12-end-to-end-flow-summary)

---

### 1. Sources and Registry
Everything starts with many different city data sources. Before we fetch anything, we keep a simple registry that says what we’re allowed to collect and under what rules. Adapters follow that charter, politely fetch bytes from each source, and bring them to a safe doorway.

### 2. Ingestion Gateway
That doorway—the ingestion gateway—checks the labels, enforces policy and fairness, prevents duplicates, and then does a clean handoff: it freezes the original bytes in an immutable raw store and stamps a small envelope that points to those bytes. That stamped envelope is appended to the Ingest Log, which is our reliable, replayable record of “what came in, and when.”

### 3. Ingest Log
From there, the Ingest Log behaves like a steady conveyor belt. It doesn’t interpret anything; it just carries the envelopes in a predictable order and keeps them available long enough for the rest of the system to read at its own pace. If anything downstream needs to pause or rerun, the log makes that safe without going back to the source, because the originals are already preserved in the vault and each envelope is stable.

### 4. Parse
The first place we actually open the box is Parse. It reads an envelope, follows the pointer to the raw bytes, and decodes the payload into neutral records. Parse knows just enough about the source to choose the right decoder—think of a small “parse plan” for format and record path—but it doesn’t change meaning. The result is a faithful stream of rows, objects, or features that still carry their provenance and a clear link back to the raw bytes and the envelope that introduced them.

### 5. Normalize
Normalize picks up those neutral records and puts them into a single, consistent geospatial shape so everything speaks the same language. Coordinates are brought into one convention, basic attributes are harmonized, and simple taxonomies are aligned. If a record only has an address, Normalize may add a coordinate through a consistent method, noting confidence and timing. It keeps lineage intact and distinguishes real‑world timing from processing timing. At this point, each record is a clean canonical feature, but we haven’t merged across sources yet—that comes later.

### 6. Quality Gate
The Quality Gate is a calm checkpoint. It looks at each canonical feature and decides whether it’s good enough for the purpose at hand. It doesn’t rewrite anything; it simply accepts, warns, or quarantines with clear reasons. Accepted features move on, questionable ones are set aside without being lost, and the rules are predictable so replays behave the same way tomorrow as they do today.

### 7. Enrich
Enrich adds just enough context to make features easy to group, find, and route. It tags each feature with the city or district it belongs to, assigns it to a lightweight spatial bin for quick lookups, and attaches any simple labels that help partition data and answer common place‑based questions. These tags are additive and versioned against the reference data they come from so you can re‑run Enrich when boundaries change and keep the story straight.

### 8. Conflation
Conflation turns many mentions into one stable thing. It looks at each enriched feature and asks, “Is this the same real‑world entity we already know about, or something new?” When it’s the same, it links the mention to the existing entity and updates that entity’s timeline; when it’s new, it assigns a fresh, durable ID. Relationships settle here too: events point to venues and artists as canonical entities rather than to raw source snippets. Conflation prefers clear matches over risky guesses and never erases history; changes close one chapter and start the next so you can ask “what was true then?” as easily as “what is true now?”

### 9. Catalog
The Catalog is the organized memory and, in your design, the final step. Canonical entities from Conflation become Nodes with durable IDs, attributes, geometry, time fields, and provenance. Relationships become Edges with their own time and provenance. Nothing is overwritten; the Catalog keeps the current view and the prior chapters. It stores place and time in ways that keep everyday questions fast, and it preserves the thread back to the exact envelopes and raw bytes that support each fact. Your GraphQL types define the shapes of these Nodes and Edges so the model is explicit and consistent. If you choose to retain mentions for audit, you can keep them as a lightweight layer that links out to the Nodes without cluttering product queries.

### 10. Operational Patterns and Storage
Around this main lane, a few operational patterns keep everything steady. The raw store holds one immutable copy of every payload. The Ingest Log is the single stream of accepted envelopes. The big, evolving record sets between stages live as partitioned, columnar files so they’re cheap to store and easy to replay. Small, transactional bits—like the source registry, run history, and indexes—live in regular SQL tables. Full step‑by‑step lineage stays in a separate operational space so audits are rich but product queries stay clean; each catalog Node carries slim pointers into that lineage when you need to dive deeper.

### 11. Core Principles
A handful of simple principles tie it all together. Same input, same outcome—retries and replays are safe. Raw bytes are immutable and everything else writes history rather than editing it. Transforms are simple and deterministic so behavior is predictable. Provenance is everywhere so any answer can explain itself. The edges are loosely coupled so new sources can join without breaking the core. And fairness at the door keeps spiky sources from overwhelming the system.

### 12. End‑to‑End Flow Summary
Taken together, this gives you a calm, explainable flow: sources to safe doorway, doorway to log, log to decode, decode to consistent shape, checkpoint for quality, add a little context, merge into stable entities, and file the truth in a graph‑shaped catalog that keeps time and provenance first‑class.

