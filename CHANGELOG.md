<!-- AUTO-GENERATED:backlink START -->
[← Back](README.md)
<!-- AUTO-GENERATED:backlink END -->
# Changelog

## 2026-08-16

- Moved Suno terms/rights import to Settings as metadata-bearing global evidence, automatically copied into new and editable projects while finalized snapshots remain untouched; raised SQLite schema to 4.
- Removed project/version ID, final generation ID, and final-generation time from current workflow inputs and generated certificate/documents; compatibility fields in existing stored records remain non-blocking.

- Raised the Suno workflow to 1.3, document templates to 1.5, SQLite schema to 4, evidence manifest schema to 2, and certificate format to 3.0.
- Added concrete final-generation metadata, instrumental/Lyrics consistency, original-filename deviation confirmation, and commercial subscription coverage checks.
- Added local `suno_terms_rights` and `external_timestamp` evidence roles with persisted factual metadata.
- Reworked PDF and Markdown certificates into the A–J technical evidence structure with statement-origin labels, earlier-revision references, deterministic PDF trailer IDs, and explicit non-legal scope.
- Preserved finalized-snapshot immutability, revision archival, relative paths, and SHA-256 integrity behavior.
