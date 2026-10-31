use serde::{Deserialize, Serialize};
use std::fmt;

/// Canonical representation of an entire database schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DatabaseSchema {
    pub name: Option<String>,
    pub tables: Vec<Table>,
}

impl DatabaseSchema {
    pub fn new(tables: Vec<Table>) -> Self {
        Self {
            name: None,
            tables,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn find_table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name.eq_ignore_ascii_case(name))
    }

    pub fn table_names(&self) -> Vec<String> {
        self.tables.iter().map(|t| t.name.clone()).collect()
    }
}

/// Representation of a database table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
    pub foreign_keys: Vec<ForeignKey>,
    pub primary_key: Vec<String>,
}

impl Table {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            primary_key: Vec::new(),
        }
    }

    pub fn find_column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    pub fn column_names(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.name.clone()).collect()
    }
}

/// Representation of a database table column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
    pub auto_increment: bool,
}

impl Column {
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: true,
            default_value: None,
            is_primary_key: false,
            auto_increment: false,
        }
    }

    pub fn not_null(mut self) -> Self {
        self.nullable = false;
        self
    }

    pub fn primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self.nullable = false;
        self
    }

    pub fn default_val(mut self, val: impl Into<String>) -> Self {
        self.default_value = Some(val.into());
        self
    }

    pub fn auto_inc(mut self) -> Self {
        self.auto_increment = true;
        self
    }
}

/// Canonical SQL Data Types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    SmallInt,
    Integer,
    BigInt,
    Float,
    Real,
    Double,
    Text,
    Varchar(Option<usize>),
    Blob,
    Boolean,
    Date,
    Timestamp,
    Json,
    Other(String),
}

impl DataType {
    pub fn from_sql(sql: &str) -> Self {
        let normalized = sql.trim().to_uppercase();
        if normalized.starts_with("INT") || normalized == "INTEGER" {
            DataType::Integer
        } else if normalized.starts_with("BIGINT") || normalized == "INT8" {
            DataType::BigInt
        } else if normalized.starts_with("SMALLINT") || normalized == "INT2" || normalized == "TINYINT" {
            DataType::SmallInt
        } else if normalized.starts_with("FLOAT") {
            DataType::Float
        } else if normalized.starts_with("REAL") {
            DataType::Real
        } else if normalized.starts_with("DOUBLE") || normalized.starts_with("NUMERIC") || normalized.starts_with("DECIMAL") {
            DataType::Double
        } else if normalized.starts_with("VARCHAR") {
            let len = sql
                .find('(')
                .and_then(|open| sql.find(')').map(|close| (open, close)))
                .and_then(|(o, c)| sql[o + 1..c].trim().parse::<usize>().ok());
            DataType::Varchar(len)
        } else if normalized.starts_with("TEXT") || normalized.starts_with("CHAR") || normalized.starts_with("STRING") {
            DataType::Text
        } else if normalized.starts_with("BLOB") || normalized.starts_with("BYTEA") || normalized.starts_with("BINARY") {
            DataType::Blob
        } else if normalized.starts_with("BOOL") {
            DataType::Boolean
        } else if normalized.starts_with("DATE") && !normalized.starts_with("DATETIME") {
            DataType::Date
        } else if normalized.starts_with("TIMESTAMP") || normalized.starts_with("DATETIME") || normalized.starts_with("TIME") {
            DataType::Timestamp
        } else if normalized.starts_with("JSON") {
            DataType::Json
        } else {
            DataType::Other(sql.to_string())
        }
    }

    pub fn sql_repr(&self) -> String {
        match self {
            DataType::SmallInt => "SMALLINT".to_string(),
            DataType::Integer => "INTEGER".to_string(),
            DataType::BigInt => "BIGINT".to_string(),
            DataType::Float => "FLOAT".to_string(),
            DataType::Real => "REAL".to_string(),
            DataType::Double => "DOUBLE".to_string(),
            DataType::Text => "TEXT".to_string(),
            DataType::Varchar(Some(len)) => format!("VARCHAR({})", len),
            DataType::Varchar(None) => "VARCHAR".to_string(),
            DataType::Blob => "BLOB".to_string(),
            DataType::Boolean => "BOOLEAN".to_string(),
            DataType::Date => "DATE".to_string(),
            DataType::Timestamp => "TIMESTAMP".to_string(),
            DataType::Json => "JSON".to_string(),
            DataType::Other(s) => s.clone(),
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sql_repr())
    }
}

/// Representation of a database index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub is_primary: bool,
}

impl Index {
    pub fn new(name: impl Into<String>, table_name: impl Into<String>, columns: Vec<String>, unique: bool) -> Self {
        Self {
            name: name.into(),
            table_name: table_name.into(),
            columns,
            unique,
            is_primary: false,
        }
    }
}

/// Representation of a database foreign key constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForeignKey {
    pub name: Option<String>,
    pub from_columns: Vec<String>,
    pub to_table: String,
    pub to_columns: Vec<String>,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

impl ForeignKey {
    pub fn new(
        from_columns: Vec<String>,
        to_table: impl Into<String>,
        to_columns: Vec<String>,
    ) -> Self {
        Self {
            name: None,
            from_columns,
            to_table: to_table.into(),
            to_columns,
            on_delete: None,
            on_update: None,
        }
    }
}
