# Envelope Contract v1 (Engineering Doc)

This document defines the envelope contract used between adapters and the ingestion gateway. It captures submission-time fields, gateway-assigned fields on acceptance, validation rules, limits, versioning, and the extension mechanism.

- Schema file: schemas/envelope.v1.json
- Examples: tests/resources/envelope_submission.json, tests/resources/envelope_accepted.json
- Validator CLI: validate-envelope (see below)

For full ticket text and rationale, see PLATONIC_IDEAL.md and this doc. 

## Top-level shape and roles
- Submission-time (adapter provides):
  - envelope_version, source_id, idempotency_key
  - payload_meta, request, timing.fetched_at (observed_at optional)
  - legal, geo_hint (optional), content.schema_hint (optional)
  - trace.trace_id (optional), trace.attempt (int, optional)
- Acceptance-time (gateway sets):
  - gateway_received_at
  - payload_ref (content-addressed pointer)
  - trace.trace_id (if missing)
  - envelope_id (gateway-assigned UUID)

## Serialization and constraints
- UTF‑8 JSON object; snake_case keys; unknown top-level keys rejected except ext.
- Envelope size: target ≤ 16 KB; hard max 64 KB.
- All timestamps are RFC3339 with Z (UTC).
- Checksums: lowercase hex sha256 of exact payload bytes as sent.
- Idempotency key: ASCII, max 256 chars; deterministic per logical slice.
- No secrets in any field. Do not include raw payload bytes in the envelope.

## Versioning policy
- Semantic versioning on envelope_version (major.minor.patch).
  - Minor/patch: backward‑compatible additions (optional fields).
  - Major: breaking changes → new schema file and topic/route.
- Unknown fields rejected except ext; forward‑compatibility via ext.

## Extension mechanism
- ext: object for experimental or source‑specific metadata.
- Keys inside ext should be namespaced, e.g., "ext": { "com.acme.connector": { ... } }.
- No gateway decisions (dedupe, policy) may rely on ext fields.

## Validation rules (beyond JSON Schema)
- envelope_version must equal 1.0.0.
- source_id must exist in Registry and be active. (CLI supports hook; gateway enforces.)
- idempotency_key deterministic per slice; gateway dedupes on exact match.
- payload_meta.size_bytes equals actual byte length received. (Gateway responsibility.)
- payload_meta.checksum.sha256 matches recomputed hash of the payload. (Gateway responsibility.)
- request.url absolute; credentials redacted in logs.
- timing.fetched_at within a configurable window of gateway_received_at (policy).
- legal.license_id allow‑listed per source; ambiguous → quarantine.
- geo_hint is advisory only; never a hard failure.
- payload_ref is gateway-only; adapters must not set it.
- ext is for namespaced experimental fields only.

## Conformance checklist
- [ ] JSON Schema validates all “happy path” examples.
- [ ] Adapters cannot set gateway-only fields.
- [ ] Validator catches: missing required fields, bad checksum length, invalid timestamps, oversize envelope, bad URL.
- [ ] Duplicate submission with same idempotency_key validates and is treated as success by gateway.
- [ ] Envelope with disallowed license flagged for quarantine (policy in gateway).
- [ ] Unit tests cover: minimal valid, full valid, invalid cases with clear messages.

## Validator CLI
- Command: validate-envelope path/to/file.json
- Exit code: 0 when valid; 1 when invalid; prints reasons.
- Implementation: Rust; uses jsonschema crate with Draft 2020‑12.


