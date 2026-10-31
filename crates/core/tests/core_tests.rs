use arrow_array::{Array, Float64Array, Int64Array, StringArray};
use strata_core::{
    diff_schemas, Column, DataType, DatabaseDriver, DatabaseSchema, DuckDbDriver, Index,
    SqlDialect, SqliteDriver, Table,
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
