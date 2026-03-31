# Changelog

## 0.1.0 - 2026-04-01

Initial release.

### Included
- manual JSON serializer
- owned parser
- borrowed parser
- tape parser
- lazy hashed object indexing
- compiled lookup keys
- compiled object and row schema serialization

### Performance direction
- strong wins on tape parsing for structural/inspection-heavy workloads
- faster repeated lookup on wide objects with indexed compiled keys
- repeated-shape serialization fast paths
