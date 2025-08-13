# Clean Architecture for sms_scraper

This document establishes the architectural conventions for this repository so new code lands in the right place and old code can be migrated incrementally without breaking builds.

Core ideas
- Dependency rule: source code dependencies always point inward. Inner layers must not know about outer layers.
- Stable center: keep the business rules (domain) the most stable and the easiest to test.
- Ports and adapters: the application layer defines traits (ports); infrastructure provides concrete implementations (adapters).

Layers
1) Domain (src/domain)
   - Pure Rust: entities, value objects, domain services, domain errors.
   - No IO, no HTTP, no filesystem, no metrics, no logging, and ideally no async.
   - Test strategy: unit tests that are fast and deterministic.

2) Application (src/application)
   - Use-cases orchestrating domain operations.
   - Defines traits (ports) for things the use-cases need: StoragePort, HttpClientPort, ClockPort, MetricsPort, IngestLogPort, etc.
   - Depends only on Domain and standard library. No concrete crates for IO here.
   - Test strategy: use mocks/fakes for ports.

3) Infrastructure (src/infrastructure)
   - Adapters that implement application ports using concrete tech: reqwest, filesystem, databases, metrics, tracing, schedulers.
   - May depend on external crates but must not leak those types into Domain or Application.
   - Test strategy: integration tests; allow using real network/filesystem in tests.

4) Interface (src/interface)
   - Delivery mechanisms: HTTP server (Axum/GraphQL), CLI, schedulers.
   - Translates external requests into application use-cases and wires adapters.
   - main.rs should be thin composition code that builds the dependency graph and runs it.

Dependency flow
Interface -> Application -> Domain
Infrastructure -> Application (implements its traits)

Migration plan
- Introduce the directories and traits (this change) without moving code.
- Extract pure types and logic from existing modules into Domain as we touch them.
- Move workflows into Application use-cases and have them depend on ports instead of concretes.
- Implement ports in Infrastructure with reqwest, filesystem, metrics, etc.
- Keep binaries (main.rs and any bins) limited to composition and argument parsing.

Conventions
- Errors: Domain and Application use thiserror-based types; Infrastructure may use anyhow internally and map at the boundary.
- Config: define a Config type at composition time; pass it to constructors when wiring.
- Testing:
  - Domain: pure unit tests.
  - Application: unit tests with mocks of ports.
  - Integration: tests/ with real adapters wired end-to-end where useful.

Adding new features
- Start at Domain: add types and invariants first.
- Define an Application use-case that expresses the workflow in domain terms and depends on ports it needs.
- Provide Infrastructure adapters that satisfy those ports.
- Wire in Interface (HTTP/CLI) and compose in main.rs.

Enforcement
- Prefer module boundaries over multi-crate initially. If boundaries hold and churn stabilizes, consider splitting into a workspace:
  - crates/domain, crates/application, crates/infrastructure, crates/interface.
- Clippy: prefer enabling warnings for common footguns; do not deny all warnings while refactoring. We can ratchet later.

