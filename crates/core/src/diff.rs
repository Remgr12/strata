use crate::schema::{Column, DatabaseSchema, Index, Table};
use serde::{Deserialize, Serialize};

/// Target SQL dialect for migration script generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlDialect {
    Sqlite,
    DuckDb,
    Generic,
}

/// The diff result between two schemas (source -> target).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SchemaDiff {
    pub added_tables: Vec<Table>,
    pub dropped_tables: Vec<Table>,
    pub altered_tables: Vec<TableDiff>,
}

impl SchemaDiff {
    pub fn is_empty(&self) -> bool {
        self.added_tables.is_empty()
            && self.dropped_tables.is_empty()
            && self.altered_tables.is_empty()
    }

    /// Generate declarative SQL migration statements to transition from schema A to schema B.
    pub fn generate_migration_sql(&self, dialect: SqlDialect) -> Vec<String> {
        let mut stmts = Vec::new();

        // 1. Dropped tables
        for tbl in &self.dropped_tables {
            stmts.push(format!("DROP TABLE IF EXISTS \"{}\";", tbl.name));
        }

        // 2. Added tables
        for tbl in &self.added_tables {
            let mut col_defs = Vec::new();
            for col in &tbl.columns {
                let mut def = format!("\"{}\" {}", col.name, col.data_type.sql_repr());
                if !col.nullable {
                    def.push_str(" NOT NULL");
                }
                if let Some(ref dv) = col.default_value {
                    def.push_str(&format!(" DEFAULT {}", dv));
                }
                col_defs.push(def);
            }

            if !tbl.primary_key.is_empty() {
                let pks = tbl
                    .primary_key
                    .iter()
                    .map(|k| format!("\"{}\"", k))
                    .collect::<Vec<_>>()
                    .join(", ");
                col_defs.push(format!("PRIMARY KEY ({})", pks));
            }

            stmts.push(format!(
                "CREATE TABLE \"{}\" (\n  {}\n);",
                tbl.name,
                col_defs.join(",\n  ")
            ));

            // Indexes for added table
            for idx in &tbl.indexes {
                if !idx.is_primary {
                    let unique_str = if idx.unique { "UNIQUE " } else { "" };
                    let cols_str = idx
                        .columns
                        .iter()
                        .map(|c| format!("\"{}\"", c))
                        .collect::<Vec<_>>()
                        .join(", ");
                    if !cols_str.is_empty() {
                        stmts.push(format!(
                            "CREATE {}INDEX IF NOT EXISTS \"{}\" ON \"{}\" ({});",
                            unique_str, idx.name, tbl.name, cols_str
                        ));
                    }
                }
            }
        }

        // 3. Altered tables
        for alter in &self.altered_tables {
            // Dropped indexes
            for idx in &alter.dropped_indexes {
                if !idx.is_primary {
                    stmts.push(format!("DROP INDEX IF EXISTS \"{}\";", idx.name));
                }
            }

            // Dropped columns
            for col in &alter.dropped_columns {
                stmts.push(format!(
                    "ALTER TABLE \"{}\" DROP COLUMN \"{}\";",
                    alter.table_name, col.name
                ));
            }

            // Added columns
            for col in &alter.added_columns {
                let mut def = format!("\"{}\" {}", col.name, col.data_type.sql_repr());
                if !col.nullable {
                    def.push_str(" NOT NULL");
                }
                if let Some(ref dv) = col.default_value {
                    def.push_str(&format!(" DEFAULT {}", dv));
                }
                stmts.push(format!(
                    "ALTER TABLE \"{}\" ADD COLUMN {};",
                    alter.table_name, def
                ));
            }

            // Altered columns
            for col_diff in &alter.altered_columns {
                match dialect {
                    SqlDialect::DuckDb => {
                        if col_diff.type_changed {
                            stmts.push(format!(
                                "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" TYPE {};",
                                alter.table_name,
                                col_diff.column_name,
                                col_diff.new_column.data_type.sql_repr()
                            ));
                        }
                        if col_diff.nullability_changed {
                            if col_diff.new_column.nullable {
                                stmts.push(format!(
                                    "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" DROP NOT NULL;",
                                    alter.table_name, col_diff.column_name
                                ));
                            } else {
                                stmts.push(format!(
                                    "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" SET NOT NULL;",
                                    alter.table_name, col_diff.column_name
                                ));
                            }
                        }
                    }
                    SqlDialect::Sqlite | SqlDialect::Generic => {
                        // SQLite does not support ALTER COLUMN directly; emit informational SQL migration comment
                        stmts.push(format!(
                            "-- [MIGRATION NOTE] Column \"{}\".\"{}\" altered from {} to {}.",
                            alter.table_name,
                            col_diff.column_name,
                            col_diff.old_column.data_type.sql_repr(),
                            col_diff.new_column.data_type.sql_repr()
                        ));
                    }
                }
            }

            // Added indexes
            for idx in &alter.added_indexes {
                if !idx.is_primary {
                    let unique_str = if idx.unique { "UNIQUE " } else { "" };
                    let cols_str = idx
                        .columns
                        .iter()
                        .map(|c| format!("\"{}\"", c))
                        .collect::<Vec<_>>()
                        .join(", ");
                    if !cols_str.is_empty() {
                        stmts.push(format!(
                            "CREATE {}INDEX IF NOT EXISTS \"{}\" ON \"{}\" ({});",
                            unique_str, idx.name, alter.table_name, cols_str
                        ));
                    }
                }
            }
        }

        stmts
    }
}

/// Changes between two versions of a table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TableDiff {
    pub table_name: String,
    pub added_columns: Vec<Column>,
    pub dropped_columns: Vec<Column>,
    pub altered_columns: Vec<ColumnDiff>,
    pub added_indexes: Vec<Index>,
    pub dropped_indexes: Vec<Index>,
}

impl TableDiff {
    pub fn is_empty(&self) -> bool {
        self.added_columns.is_empty()
            && self.dropped_columns.is_empty()
            && self.altered_columns.is_empty()
            && self.added_indexes.is_empty()
            && self.dropped_indexes.is_empty()
    }
}

/// Changes between two versions of a column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnDiff {
    pub column_name: String,
    pub old_column: Column,
    pub new_column: Column,
    pub type_changed: bool,
    pub nullability_changed: bool,
    pub default_changed: bool,
}

/// Compute declarative difference from schema `a` (source / base) to schema `b` (target / current).
pub fn diff_schemas(a: &DatabaseSchema, b: &DatabaseSchema) -> SchemaDiff {
    let mut diff = SchemaDiff::default();

    // 1. Detect added tables (present in b, not in a)
    for b_table in &b.tables {
        if a.find_table(&b_table.name).is_none() {
            diff.added_tables.push(b_table.clone());
        }
    }

    // 2. Detect dropped tables (present in a, not in b)
    for a_table in &a.tables {
        if b.find_table(&a_table.name).is_none() {
            diff.dropped_tables.push(a_table.clone());
        }
    }

    // 3. Detect altered tables (present in both)
    for a_table in &a.tables {
        if let Some(b_table) = b.find_table(&a_table.name) {
            let mut tbl_diff = TableDiff {
                table_name: a_table.name.clone(),
                ..Default::default()
            };

            // Added columns
            for b_col in &b_table.columns {
                if a_table.find_column(&b_col.name).is_none() {
                    tbl_diff.added_columns.push(b_col.clone());
                }
            }

            // Dropped columns
            for a_col in &a_table.columns {
                if b_table.find_column(&a_col.name).is_none() {
                    tbl_diff.dropped_columns.push(a_col.clone());
                }
            }

            // Altered columns
            for a_col in &a_table.columns {
                if let Some(b_col) = b_table.find_column(&a_col.name) {
                    let type_changed = a_col.data_type != b_col.data_type;
                    let nullability_changed = a_col.nullable != b_col.nullable;
                    let default_changed = a_col.default_value != b_col.default_value;

                    if type_changed || nullability_changed || default_changed {
                        tbl_diff.altered_columns.push(ColumnDiff {
                            column_name: a_col.name.clone(),
                            old_column: a_col.clone(),
                            new_column: b_col.clone(),
                            type_changed,
                            nullability_changed,
                            default_changed,
                        });
                    }
                }
            }

            // Added indexes
            for b_idx in &b_table.indexes {
                if !a_table.indexes.iter().any(|i| i.name == b_idx.name) {
                    tbl_diff.added_indexes.push(b_idx.clone());
                }
            }

            // Dropped indexes
            for a_idx in &a_table.indexes {
                if !b_table.indexes.iter().any(|i| i.name == a_idx.name) {
                    tbl_diff.dropped_indexes.push(a_idx.clone());
                }
            }

            if !tbl_diff.is_empty() {
                diff.altered_tables.push(tbl_diff);
            }
        }
    }

    diff
}
