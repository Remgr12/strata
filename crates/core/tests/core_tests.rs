use arrow_array::{Array, Float64Array, Int64Array, StringArray};
use strata_core::{
    diff_schemas, is_newer_version, parse_sha256sums, parse_version, select_asset_for_install,
    Column, DataType, DatabaseDriver, DatabaseSchema, DuckDbDriver, Index, InstallKind,
    ReleaseAsset, ReleaseInfo, SqlDialect, SqliteDriver, Table,
};

#[tokio::test]
async fn test_sqlite_driver_lifecycle() {
    let mut driver = SqliteDriver::new();
    driver
        .connect(":memory:")
        .await
        .expect("Failed to connect SQLite in-memory");
    assert!(driver.is_connected());
    assert_eq!(driver.engine_name(), "sqlite");

    // 1. Create table
    let ddl = "CREATE TABLE users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        email TEXT UNIQUE,
        balance REAL DEFAULT 0.0
    );";
    let res = driver.execute_query(ddl).await.expect("Failed to run DDL");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].num_rows(), 1);

    // 2. Insert records
    let insert_sql = "INSERT INTO users (name, email, balance) VALUES ('Alice', 'alice@strata.dev', 1250.50), ('Bob', 'bob@strata.dev', 300.0);";
    let insert_res = driver
        .execute_query(insert_sql)
        .await
        .expect("Failed to run insert");
    assert_eq!(insert_res.len(), 1);

    // 3. Query records into Arrow RecordBatches
    let select_sql = "SELECT id, name, email, balance FROM users ORDER BY id ASC;";
    let batches = driver
        .execute_query(select_sql)
        .await
        .expect("Failed to execute SELECT");
    assert_eq!(batches.len(), 1);
    let batch = &batches[0];
    assert_eq!(batch.num_rows(), 2);
    assert_eq!(batch.num_columns(), 4);

    let id_col = batch
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .expect("id col is Int64");
    assert_eq!(id_col.value(0), 1);
    assert_eq!(id_col.value(1), 2);

    let name_col = batch
        .column(1)
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("name col is String");
    assert_eq!(name_col.value(0), "Alice");
    assert_eq!(name_col.value(1), "Bob");

    let balance_col = batch
        .column(3)
        .as_any()
        .downcast_ref::<Float64Array>()
        .expect("balance col is Float64");
    assert_eq!(balance_col.value(0), 1250.50);

    // 4. Introspect schema
    let schema = driver
        .introspect_schema()
        .await
        .expect("Failed to introspect schema");
    assert_eq!(schema.tables.len(), 1);
    let table = &schema.tables[0];
    assert_eq!(table.name, "users");
    assert_eq!(table.columns.len(), 4);

    let name_col_schema = table.find_column("name").expect("Column name exists");
    assert_eq!(name_col_schema.data_type, DataType::Text);
    assert!(!name_col_schema.nullable);

    let id_col_schema = table.find_column("id").expect("Column id exists");
    assert!(id_col_schema.is_primary_key);
}

#[tokio::test]
async fn test_schema_diff_and_migration_generation() {
    // Base Schema A: 1 table with 2 columns
    let table_a = Table {
        name: "orders".to_string(),
        columns: vec![
            Column::new("id", DataType::Integer).primary_key(),
            Column::new("customer", DataType::Text).not_null(),
        ],
        indexes: vec![],
        foreign_keys: vec![],
        primary_key: vec!["id".to_string()],
    };
    let schema_a = DatabaseSchema::new(vec![table_a]);

    // Target Schema B: "orders" table altered + new "products" table added
    let table_b_orders = Table {
        name: "orders".to_string(),
        columns: vec![
            Column::new("id", DataType::Integer).primary_key(),
            Column::new("customer", DataType::Text).not_null(),
            // Added column:
            Column::new("total_price", DataType::Double).default_val("0.0"),
        ],
        indexes: vec![Index::new(
            "idx_orders_customer",
            "orders",
            vec!["customer".to_string()],
            false,
        )],
        foreign_keys: vec![],
        primary_key: vec!["id".to_string()],
    };

    let table_b_products = Table {
        name: "products".to_string(),
        columns: vec![
            Column::new("id", DataType::Integer).primary_key(),
            Column::new("title", DataType::Text).not_null(),
        ],
        indexes: vec![],
        foreign_keys: vec![],
        primary_key: vec!["id".to_string()],
    };

    let schema_b = DatabaseSchema::new(vec![table_b_orders, table_b_products]);

    // Diff schemas
    let diff = diff_schemas(&schema_a, &schema_b);
    assert!(!diff.is_empty());
    assert_eq!(diff.added_tables.len(), 1);
    assert_eq!(diff.added_tables[0].name, "products");
    assert_eq!(diff.dropped_tables.len(), 0);
    assert_eq!(diff.altered_tables.len(), 1);

    let altered = &diff.altered_tables[0];
    assert_eq!(altered.table_name, "orders");
    assert_eq!(altered.added_columns.len(), 1);
    assert_eq!(altered.added_columns[0].name, "total_price");
    assert_eq!(altered.added_indexes.len(), 1);

    // Verify generated migration statements
    let migrations = diff.generate_migration_sql(SqlDialect::Sqlite);
    let combined_sql = migrations.join("\n");
    assert!(combined_sql.contains("CREATE TABLE \"products\""));
    assert!(combined_sql.contains("ALTER TABLE \"orders\" ADD COLUMN \"total_price\" DOUBLE DEFAULT 0.0;"));
    assert!(combined_sql.contains("CREATE INDEX IF NOT EXISTS \"idx_orders_customer\" ON \"orders\" (\"customer\");"));
}

#[tokio::test]
async fn test_duckdb_driver_lifecycle() {
    let mut driver = DuckDbDriver::new();
    driver
        .connect(":memory:")
        .await
        .expect("Failed to connect DuckDB in-memory");
    assert!(driver.is_connected());
    assert_eq!(driver.engine_name(), "duckdb");

    // Execute query
    let sql = "SELECT 42 AS num, 'Strata' AS project, true AS is_fast;";
    let batches = driver
        .execute_query(sql)
        .await
        .expect("Failed to execute DuckDB query");
    assert!(!batches.is_empty());
    let batch = &batches[0];
    assert_eq!(batch.num_rows(), 1);
    assert_eq!(batch.num_columns(), 3);
}

#[test]
fn test_updater_version_comparison_and_asset_selection() {
    assert_eq!(parse_version("v0.0.3.1"), vec![0, 0, 3, 1]);
    assert_eq!(parse_version("0.0.4"), vec![0, 0, 4]);
    assert!(is_newer_version("0.0.3.1", "v0.0.4"));
    assert!(is_newer_version("0.0.3", "0.0.3.1"));
    assert!(!is_newer_version("0.0.4", "0.0.3.1"));
    assert!(!is_newer_version("0.0.4", "v0.0.4"));

    let release = ReleaseInfo {
        tag_name: "v0.0.4".to_string(),
        name: "Strata v0.0.4".to_string(),
        body: "Release notes".to_string(),
        html_url: "https://git.soventa.tech/moha/strata/releases/tag/v0.0.4".to_string(),
        prerelease: false,
        draft: false,
        assets: vec![
            ReleaseAsset {
                name: "strata-linux".to_string(),
                browser_download_url: "https://example.com/strata-linux".to_string(),
                size: 100,
            },
            ReleaseAsset {
                name: "strata-linux-x86_64.flatpak".to_string(),
                browser_download_url: "https://example.com/strata-linux-x86_64.flatpak".to_string(),
                size: 200,
            },
            ReleaseAsset {
                name: "strata-windows-x86_64.msi".to_string(),
                browser_download_url: "https://example.com/strata-windows-x86_64.msi".to_string(),
                size: 300,
            },
            ReleaseAsset {
                name: "strata-windows-x86_64.exe".to_string(),
                browser_download_url: "https://example.com/strata-windows-x86_64.exe".to_string(),
                size: 400,
            },
        ],
    };

    let msi = select_asset_for_install(&release, InstallKind::WindowsMsi).unwrap();
    assert_eq!(msi.name, "strata-windows-x86_64.msi");

    let exe = select_asset_for_install(&release, InstallKind::WindowsPortable).unwrap();
    assert_eq!(exe.name, "strata-windows-x86_64.exe");

    let flatpak = select_asset_for_install(&release, InstallKind::LinuxFlatpak).unwrap();
    assert_eq!(flatpak.name, "strata-linux-x86_64.flatpak");

    let linux = select_asset_for_install(&release, InstallKind::LinuxNative).unwrap();
    assert_eq!(linux.name, "strata-linux");

    let sums = parse_sha256sums(
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  strata-linux\n\
         abcd1234  *strata-windows-x86_64.msi\n",
    );
    assert_eq!(
        sums.get("strata-linux").map(|s| s.as_str()),
        Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    );
    assert_eq!(
        sums.get("strata-windows-x86_64.msi").map(|s| s.as_str()),
        Some("abcd1234")
    );
}
