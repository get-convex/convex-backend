We're starting to use api-extractor for api reports. The saved versions are
committed to reports/ directory, making it possible to track evolution of the
public API.

Since api-extractor .d.ts rollups don't support declaration map, using these as
published types break jump-to-definition in VSCode. Until
[declaration map rollups](https://github.com/microsoft/rushstack/issues/1886)
are implemented we compile public types with `tsc --stripInternal`, which
requires marking exports as internal at the index.ts barrel file level if they
need to be used in multiple files.
