<div align="center">
  <h1>strata</h1>

  <p><strong>A native, GPU-accelerated local database client and schema diffing utility.</strong></p>

  <p>
    <a href="#credits"><img alt="Lead developer" src="https://img.shields.io/badge/lead%20developer-Zsombor%20Simon-58a9ff" /></a>
    <a href="https://git.soventa.tech/moha/strata"><img alt="Repository" src="https://img.shields.io/badge/forgejo-git.soventa.tech-58a9ff" /></a>
    <a href="https://git.soventa.tech/moha/strata/releases"><img alt="Release" src="https://img.shields.io/badge/release-v0.0.1-58a9ff" /></a>
    <a href="#license"><img alt="License" src="https://img.shields.io/badge/license-MIT-9e6bfa" /></a>
  </p>
</div>

strata is a native, GPU-accelerated local database client and schema diffing utility built around a modular core engine written in 100% pure Rust and Zed's GPUI framework. Specialized for embedded data files—SQLite, libSQL, DuckDB, and Apache Parquet—it delivers sub-50ms startup times, a minimal memory footprint (<60MB RSS), and fluid 120 FPS rendering via Vulkan and Metal without web runtimes or Electron bloat.

## Features

- **Zero-copy columnar grid** - virtualized table rendering directly from Apache Arrow `RecordBatch` columnar arrays with sub-millisecond scrolling and zero memory duplication
- **Multi-engine local drivers** - unified query and introspection interface for SQLite (`rusqlite`), DuckDB, libSQL, and Apache Parquet files
- **Canonical Schema AST** - rich structural representation of database schemas including tables, typed column mappings, primary keys, indexes, and foreign keys
- **Declarative schema diffing** - structural comparison between snapshots or live schemas detecting added tables, dropped tables, and altered column definitions
- **Automated migration generator** - declarative `ALTER TABLE` and `CREATE TABLE` migration SQL script generation tailored for SQLite and DuckDB dialects
- **Drag-and-drop file ingestion** - immediate file inspection and schema introspection by dropping `.sqlite`, `.db`, `.duckdb`, or `.parquet` files onto the window
- **Multi-line SQL console** - responsive SQL editor with execution status, error reporting, sub-millisecond execution timers, and `Cmd/Ctrl + Enter` execution
- **Collapsible schema tree** - interactive explorer sidebar displaying tables, column data types, primary key badges, and table column counts
- **Dual-tab workflow** - frictionless switching between the live query data grid and visual schema diff / migration panel
- **Pure native GPU rendering** - Zed GPUI framework rendering directly via Vulkan (Linux/Windows) and Metal (macOS)
- **Non-blocking asynchronous I/O** - background task executor (`cx.spawn`) keeping the 120 FPS UI completely stutter-free during heavy disk I/O and SQL queries
- **Zero UI dependencies in core** - headless `crates/core` domain library fully decoupled from UI frameworks for CLI, server, and test automation

## Technology Stack

| Layer | Technology |
|-------|-----------|
| UI Framework | Zed GPUI (`gpui 0.2.2`) - GPU-accelerated native rendering (Metal / Vulkan) |
| UI Components | `gpui-component 0.8.1` + virtualized uniform lists |
| Async Runtime | GPUI Background Executor (`cx.background_executor()`, `AsyncApp`) |
| Data Representation | Apache Arrow 60.1 (`arrow`, `arrow-array`, `arrow-schema`) |
| Core Engine | Rust (`crates/core`) - Headless driver traits, AST, schema diffing |
| SQLite Engine | `rusqlite 0.33` (bundled C-SQLite 3.49) |
| OLAP Engine | `duckdb 1.2` (embedded DuckDB with native Parquet reader) |
| Styling | GPUI functional Tailwind-style builder API + Catppuccin Macchiato theme |

## Project Structure

```
strata/
├── Cargo.toml                # Rust workspace configuration
├── crates/
│   ├── core/                 # Pure headless domain library (zero GPUI dependency)
│   │   ├── src/
│   │   │   ├── driver.rs     # Async DatabaseDriver trait (connect, introspect, query)
│   │   │   ├── sqlite.rs     # SQLite driver with dynamic Arrow RecordBatch builders
│   │   │   ├── duckdb.rs     # DuckDB & Parquet driver with zero-copy Arrow streaming
│   │   │   ├── schema.rs     # Canonical Schema AST (Table, Column, DataType, Index, FK)
│   │   │   ├── diff.rs       # Schema diffing & declarative SQL migration generator
│   │   │   └── lib.rs        # Core public API exports
│   │   └── tests/
│   │       └── core_tests.rs # Integration tests for SQLite, DuckDB, and diffing
│   └── app/                  # GPUI Desktop Application
│       └── src/
│           ├── state.rs      # Reactive AppState entity & background task coordination
│           ├── views/        # GPUI presentation views
│           │   ├── table_view.rs  # Virtualized Arrow data grid (uniform_list)
│           │   ├── query_view.rs  # SQL editor & execution controls
│           │   ├── schema_tree.rs # Collapsible schema & explorer sidebar
│           │   ├── diff_view.rs   # Visual schema diff modal & migration panel
│           │   └── mod.rs
│           └── main.rs       # GPUI application lifecycle, window setup & file drop
└── patches/                  # Patched dependencies (e.g. xattr for Linux aarch64)
```

## Keybinds

| Keys | Action |
|------|--------|
| `Ctrl Enter` / `Cmd Enter` | Execute SQL Query |
| `Ctrl 1` / `Cmd 1` | Switch to Data Grid / Query View |
| `Ctrl 2` / `Cmd 2` | Switch to Schema Diff / Migration View |
| `Esc` | Clear selection / dismiss modals |

All keybinds are globally registered on desktop platforms for rapid database querying and schema inspection.

## Building

### Prerequisites

- Rust toolchain (`rustup`, 1.80+)
- System dependencies (Linux):
  - `libxkbcommon-dev`
  - `libxkbcommon-x11-dev`
  - `libxcb1-dev`
  - `libfreetype6-dev`
  - Vulkan or OpenGL drivers

### Development

```bash
git clone https://git.soventa.tech/moha/strata.git
cd strata

# Launch the desktop app
cargo run -p strata-app
```

### Running Tests

```bash
# Run all workspace unit and integration tests
cargo test --workspace

# Run core engine tests in isolation
cargo test -p strata-core
```

### Production Build

```bash
# Produces standalone optimized release binary
cargo build --release -p strata-app
```

## Release History

- **v0.0.1** - Initial release of Strata: modular workspace architecture (`crates/core` + `crates/app`), SQLite & DuckDB/Parquet drivers with zero-copy Apache Arrow integration (STRATA-4, STRATA-5), canonical schema AST (STRATA-2), declarative migration generator (STRATA-6), GPUI virtualized uniform list data grid (STRATA-9), multi-line SQL editor (STRATA-9), collapsible schema tree explorer (STRATA-9), visual schema diffing tab (STRATA-9), reactive background async coordination (STRATA-8), and window drag-and-drop file ingestion (STRATA-10).

## Roadmap

- **STRATA-12** - libSQL remote vector extension and replica synchronization
- **STRATA-13** - Interactive ER diagram visualization using GPUI vector canvas
- **STRATA-14** - Parquet multi-file directory partition filtering and streaming export

## License

Licensed under the MIT License:

- [LICENSE](LICENSE)

## Links

- [Documentation](https://docs.soventa.tech/books/strata)
- [Issue Tracker](https://tasks.soventa.tech/projects/3)
- [GPUI Framework](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [Apache Arrow](https://arrow.apache.org/)
