# Repository Guidelines

Guidelines for contributors to **acadrust**, a pure Rust library for reading and writing DXF and DWG CAD files (MPL-2.0).

## Project Structure & Module Organization

- `src/` — library source. Domain modules mirror DXF/DWG concepts: `entities/` (one file per entity, e.g. `line.rs`, `hatch.rs`), `objects/`, `tables/`, `types/`, `xdata/`, plus shared `document.rs`, `error.rs`, and `lib.rs`.
- `src/io/` — format codecs: `dxf/`, `dwg/`, and `import/` (STL, OBJ, FBX, COLLADA, glTF).
- `tests/` — integration tests (one file per feature or regression, e.g. `roundtrip.rs`).
- `examples/` — runnable usage and diagnostics, e.g. `roundtrip_analysis.rs`.
- `Cargo.toml` — features: `serde` (serialization) and `import` (XML/COLLADA).
- Reference: [OpenDesign DWG spec](src/docs/OpenDesign_Specification_for_.dwg_files.pdf).

## Build, Test, and Development Commands

- `cargo build` — compile the library.
- `cargo test` — run unit and integration tests.
- `cargo test --all-features` — run tests with `serde` and `import` enabled.
- `cargo clippy --all-targets` — lint; treat warnings as errors before pushing.
- `cargo fmt --check` — verify formatting.
- `cargo doc --no-deps` — build API docs.
- `cargo run --example roundtrip_analysis` — run a round-trip analysis example.

## Coding Style & Naming Conventions

- Format with `cargo fmt` (default rustfmt settings: 4-space indent, `snake_case` files/modules, `CamelCase` types, `SCREAMING_SNAKE_CASE` constants).
- Use `#![deny(missing_docs)]` conventions: document public items.
- Follow Rust 2021 idioms; prefer `Result`/`thiserror` for library errors and avoid `unwrap` in library code paths.
- Group related parsing/writing logic by module; keep entity-specific code in its own file.

## Testing Guidelines

- Unit tests live next to source; integration tests in `tests/`.
- Name tests descriptively in `snake_case` (e.g. `table_dwg_roundtrip`), and keep `*_roundtrip` tests byte-faithful where promised.
- Every parser or writer change should include a round-trip test and, where relevant, a version matrix (R13–R2018).
- Run `cargo test --all-features` before opening a pull request.
- Multi-version DWG output is exercised by `tests/dwg_output_matrix.rs` (writes all 8 versions, verifies read-back; ODA verification is opt-in via `ODA_FILE_CONVERTER` + `-- --ignored`). See `docs/dwg-output-oda-verification.md` for the verification matrix and the known R13/R14 limitation.

## Commit & Pull Request Guidelines

- Use Conventional Commits with scoped subjects: `fix(dwg): ...`, `feat(acis): ...`, `test(dxf): ...`, `chore: ...` (see Git history).
- Update `Cargo.toml` version and the Changelog section in `README.md` for release-worthy changes.
- Pull requests: describe the change and motivation, link related issues, and summarize verification (tests run, round-trip results).

## Agent-Specific Instructions

- Read `README.md` before touching public API; keep API and docs in sync.
- Prefer additive changes; preserve `CadDocument` handle-based entity storage and existing public signatures.
- Do not modify `Cargo.lock` unless dependency changes require it.
