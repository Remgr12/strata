<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/dark-mode.svg" />
    <source media="(prefers-color-scheme: light)" srcset="assets/light-mode.svg" />
    <img alt="strata" src="assets/dark-mode.svg" width="160" />
  </picture>

  <h1>strata</h1>

  <p><strong>A native, GPU-accelerated local database client and schema diffing utility.</strong></p>

  <p>
    <a href="#credits"><img alt="Lead developer" src="https://img.shields.io/badge/lead%20developer-Zsombor%20Simon-58a9ff" /></a>
    <a href="https://git.soventa.tech/moha/strata"><img alt="Repository" src="https://img.shields.io/badge/forgejo-git.soventa.tech-58a9ff" /></a>
    <a href="https://git.soventa.tech/moha/strata/releases"><img alt="Release" src="https://img.shields.io/badge/release-v0.0.5-58a9ff" /></a>
    <a href="#license"><img alt="License" src="https://img.shields.io/badge/license-MIT-9e6bfa" /></a>
  </p>
</div>

strata is a native, GPU-accelerated local database client and schema diffing utility built around a modular core engine written in 100% pure Rust and Zed's GPUI framework. Specialized for embedded data files—SQLite, libSQL, DuckDB, and Apache Parquet—it delivers sub-50ms startup times, a minimal memory footprint (<60MB RSS), and fluid 120 FPS rendering via Vulkan, DirectX, and Metal without web runtimes or Electron bloat.

## Features

- **Zero-copy columnar grid** - virtualized table rendering directly from Apache Arrow `RecordBatch` columnar arrays with sub-millisecond scrolling, row clipboard export, and zero memory duplication
- **Multi-engine local drivers** - unified query and introspection interface for SQLite (`rusqlite`), DuckDB, libSQL, and Apache Parquet files
- **Canonical Schema AST** - rich structural representation of database schemas including tables, typed column mappings, primary keys, indexes, and foreign keys
- **Declarative schema diffing** - structural comparison between live snapshots or external database files detecting added tables, dropped tables, and altered column definitions
- **Automated migration generator** - declarative `ALTER TABLE` and `CREATE TABLE` migration SQL script generation tailored for SQLite and DuckDB dialects
- **Integrated custom titlebar & window controls** - decluttered obsidian workspace with native-behaving Minimize, Maximize/Restore, and Close controls integrated directly into the top taskbar
- **Interactive SQL console & file dialogs** - focusable multi-line SQL editor with line numbers, clipboard paste/copy, quick query templates, native OS file pickers (`Ctrl+O`), and drag-and-drop file ingestion
- **Built-in multi-platform auto-updater** - periodic background release polling against Forgejo Releases with SHA-256 verification and automatic updates for Windows MSI/EXE, native Linux binaries, and Linux Flatpak bundles
- **Standalone Windows & Flatpak packaging** - static MinGW C++/GCC/winpthread runtime bundling into `strata.exe` (no external DLL errors or console popups) alongside MSI installers and Linux Flatpak (`tech.soventa.strata`) bundles

## Technology Stack

| Layer | Technology |
|-------|-----------|
| UI Framework | Zed GPUI (`gpui 0.2.2`) - GPU-accelerated native rendering (Vulkan / DirectX / Metal) |
| Async Runtime | GPUI Background Executor (`cx.background_executor()`, `AsyncApp`) |
| Data Representation | Apache Arrow 60.1 (`arrow`, `arrow-array`, `arrow-schema`) |
| Core Engine | Rust (`crates/core`) - Headless driver traits, AST, schema diffing, release updater |
| SQLite Engine | `rusqlite 0.33` (bundled C-SQLite 3.49) |
| OLAP Engine | `duckdb 1.2` (embedded DuckDB with native Parquet reader) |
| Packaging | Windows MSI (`wixl`) + Static EXE, Linux Native ELF + Flatpak (`tech.soventa.strata`) |

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
│   │   │   ├── updater.rs    # Periodic release checker & self-updater (Windows, Linux, Flatpak)
│   │   │   └── lib.rs        # Core public API exports
│   │   └── tests/
│   │       └── core_tests.rs # Integration tests for SQLite, DuckDB, diffing, and updater
│   └── app/                  # GPUI Desktop Application
│       ├── build.rs          # Windows resource embedding & static MinGW runtime bundling
│       └── src/
│           ├── state.rs      # Reactive AppState entity, file pickers & periodic update polling
│           ├── views/        # GPUI presentation views
│           │   ├── table_view.rs  # Virtualized Arrow data grid (uniform_list)
│           │   ├── query_view.rs  # Interactive focusable SQL editor & action bar
│           │   ├── schema_tree.rs # Connections & collapsible schema explorer sidebar
│           │   ├── diff_view.rs   # Split-pane schema comparison & migration SQL view
│           │   └── mod.rs
│           └── main.rs       # Custom titlebar with integrated window buttons & keybindings
├── packaging/
│   ├── flatpak/              # Flatpak manifest, AppStream metainfo & desktop entry
│   ├── linux/                # Native Linux XDG desktop entry
│   └── windows/              # WiX MSI installer definition, Win32 manifest & icon resources
└── scripts/
    ├── build-flatpak.sh      # Flatpak OSTree & single-file .flatpak bundle builder
    ├── patch-gpui.sh         # GPUI DirectX runtime shader compiler patch for MinGW
    └── publish-release.py    # Forgejo release publisher with SHA256SUMS generation
```

## Keybinds

| Keys | Action |
|------|--------|
| `Ctrl Enter` / `Cmd Enter` | Execute SQL Query |
| `Ctrl O` / `Cmd O` | Open Database or Parquet File |
| `Ctrl 1` / `Cmd 1` | Switch to Workspace (SQL & Data Grid) |
| `Ctrl 2` / `Cmd 2` | Switch to Schema Diff & Migrations |
| `Ctrl U` | Check for Updates |

## Building & Packaging

### Prerequisites

- Rust toolchain (`rustup`, 1.80+)
- System dependencies (Linux):
  - `libxkbcommon-dev`, `libxkbcommon-x11-dev`
  - `libxcb1-dev`, `libfreetype6-dev`, `libfontconfig1-dev`, `libvulkan-dev`
  - Optional (for Flatpak bundle creation): `flatpak`, `flatpak-builder`, `ostree`

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
```

### Building Linux Flatpak Bundle

```bash
cargo build --release -p strata-app
mkdir -p dist && cp target/release/strata-app dist/strata-linux
./scripts/build-flatpak.sh dist/strata-linux dist/strata-linux-x86_64.flatpak x86_64

# Install the Flatpak bundle locally:
flatpak install --user -y --bundle dist/strata-linux-x86_64.flatpak
```

## Release History

- **v0.0.5** - CI runner cache pruning and build pipeline maintenance:
  - Add automated cache cleaning steps to Linux and Windows CI workflows to prune stale incremental artifacts, intermediate DuckDB source trees, and actcache runner blobs.
- **v0.0.4** - Complete UI redesign & cross-platform distribution upgrade:
  - **Windows static runtime bundling & GUI subsystem**: Statically link `libwinpthread-1.dll`, `libstdc++-6.dll`, and `libgcc_s_seh-1.dll` directly into `strata.exe` via `crates/app/build.rs`, strip `.rsrc` version headers from bundled MSI DLL copies so Windows Installer never skips `libwinpthread-1.dll` on upgrade, and configure `windows_subsystem = "windows"` so no `cmd.exe` console window appears on launch.
  - **Linux Flatpak support**: Add `tech.soventa.strata` Flatpak manifest, AppStream metainfo, desktop integration, and automated `.flatpak` single-file bundle creation in CI.
  - **Periodic release updater**: Background release polling (`crates/core/src/updater.rs`) against Forgejo Releases with SHA-256 checksum verification and 1-click self-update support for Windows MSI/EXE, native Linux binaries, and Flatpak installations (`flatpak-spawn --host`).
  - **Complete UI overhaul**: Remove upper taskbar title text and marketing clutter badges (`<60MB RAM`, `<50ms Boot`, `GPU Native`), build native-behaving Minimize, Maximize/Restore, and Close window buttons into the top taskbar, add interactive keyboard editing to the SQL editor, add native file open/compare dialogs, and introduce a cohesive dark obsidian design system.
- **v0.0.3.1** - Windows installer & runtime hotfix: fix MSI package version injection in CI workflow, update MSI installer version to 5.0, and embed Windows application manifest requesting Common Controls v6.0.0.0.
- **v0.0.3** - Brand identity overhaul and desktop icon assets: integrate dark-mode and light-mode vector logos and multi-resolution desktop icon sets.
- **v0.0.2** - Initial Windows MSI installer definition and MinGW packaging.
- **v0.0.1** - Initial release of Strata.

## License

Licensed under the MIT License:

- [LICENSE](LICENSE)

## Links

- [Documentation](https://docs.soventa.tech/books/strata)
- [Issue Tracker](https://tasks.soventa.tech/projects/3)
- [GPUI Framework](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [Apache Arrow](https://arrow.apache.org/)
