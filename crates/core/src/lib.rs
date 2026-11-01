pub mod diff;
pub mod driver;
pub mod duckdb;
pub mod schema;
pub mod sqlite;
pub mod updater;

pub use diff::{diff_schemas, ColumnDiff, SchemaDiff, SqlDialect, TableDiff};
pub use driver::{DatabaseDriver, DriverResult};
pub use duckdb::DuckDbDriver;
pub use schema::{Column, DataType, DatabaseSchema, ForeignKey, Index, Table};
pub use sqlite::SqliteDriver;
pub use updater::{
    check_for_updates, check_for_updates_with_url, compute_file_sha256,
    download_and_apply_update, is_newer_version, parse_sha256sums, parse_version,
    select_asset_for_install, InstallKind, ReleaseAsset, ReleaseInfo, UpdateCheckResult,
    DEFAULT_RELEASES_API_URL, DEFAULT_RELEASES_PAGE_URL, FLATPAK_APP_ID,
    UPDATE_CHECK_INTERVAL_SECS,
};
