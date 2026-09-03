# X-Native Design Reset

This archive intentionally removes the previous designer UI implementation and design-specific reference material so a new interface can be designed from a clean surface.

Removed:
- x_native_app UI shell (chrome/theme/icons/helpers/demo/app state wiring)
- X-Native design-system documents
- bundled demo assets
- export fixture images

Preserved:
- x-core engine and document model
- x-editor interaction/geometry systems
- x-render/Vello rendering infrastructure
- x-text typography/shaping/cache
- x-format import/export/serialization
- x-components
- libraries, variables, assets, components, prototype engine
- tests, benchmarks, reliability and persistence infrastructure

Goal: build a new X-Native designer workspace without inheriting the old UI's visual decisions.
