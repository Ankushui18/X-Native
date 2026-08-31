# .x File Format — v2 Specification (FROZEN)

Status: **frozen contract** as of engine 0.17. Changes require a version
bump and a migration function. No field may change meaning within v2.

## Top-level shape

```json
{
  "format": "x-native",          // REQUIRED, exact string
  "version": 2,                  // REQUIRED, integer schema version
  "metadata": { ... },           // OPTIONAL, unknown keys preserved
  "variables": { ... },          // OPTIONAL
  "styles": { ... },             // OPTIONAL (reserved, v2 defines shape)
  "fonts": [ ... ],              // OPTIONAL
  "assets": [ ... ],             // OPTIONAL
  "prototypes": { ... },         // OPTIONAL (reserved)
  "pages": [ Node, ... ]         // REQUIRED (may be empty)
}
```

### metadata
```json
{ "name": "My File", "created": "<rfc3339>", "modified": "<rfc3339>",
  "app_version": "0.17.0", "uuid": "<document uuid>" }
```

### fonts
Font *references*, never embedded data:
```json
[ { "family": "DejaVuSans", "style": "Regular", "source": "system" } ]
```

### assets
Content-addressed references. Binary payloads live NEXT to the .x file
(sidecar `assets/<sha>.png`) or inline as base64 (small files only):
```json
[ { "id": "checker", "kind": "png", "sha256": "<hex>", "href": "assets/checker.png" } ]
```

### Node (unchanged from v1, plus)
- `uuid`: stable identity. v2 files SHOULD carry one per node; the loader
  generates deterministic UUIDs for legacy nodes (see UUID strategy).
- All v1 node fields keep their exact v1 meaning.

## Stable IDs / UUID strategy
- `Node.id` remains the human-readable working id (used by overrides).
- `Node.uuid` is a 128-bit hex string, assigned once, never regenerated on
  save. Loaders must preserve unknown uuids byte-for-byte.
- Deterministic backfill for v1 files: `uuid = fnv1a128(path-from-root)`,
  so migrating the same v1 file twice yields identical uuids.

## Deterministic serialization
- Object keys are emitted in a fixed order (schema order).
- All maps (variables, overrides, bindings, collections, modes) sort keys.
- Floats: shortest round-trip representation, no trailing zeros, `-0 == 0`.
- Guarantee: `save(load(save(d))) == save(d)` byte-for-byte.

## Versioning & migration
- Loader accepts any `version <= CURRENT` and migrates stepwise
  (v1→v2→…), each step a pure function `migrate_vN_to_vN+1(json)`.
- `version > CURRENT` is a hard error naming both versions.
- v1→v2 migration: wraps metadata, backfills uuids, moves nothing else.

## Validation
`validate(doc)` returns a list of structured issues, never panics:
- E001 duplicate node id (within a page)
- E002 instance references missing component
- E003 override targets id not present in the referenced master
- E004 variable binding references undefined variable
- E005 prototype destination page missing
- E006 negative/NaN geometry
- W101 asset reference without sidecar (warning)

## Corruption recovery
`load_x_lenient(text)` never fails outright:
- Trailing garbage / truncated tail: parse the longest valid prefix by
  brace balancing, report what was dropped.
- Bad node entries: skipped individually, siblings survive.
- Unknown `kind`: preserved as an empty Group named after the id.
- Result: `(Document, Vec<RecoveryNote>)`.

## Partial loading
Page payloads can be located and decoded independently:
- `list_pages(text) -> Vec<(id, byte_range)>` scans without full parse.
- `load_page(text, id)` decodes only that page's subtree.
- Enables thumbnails, lazy multi-page files, and crash-safe previews.
