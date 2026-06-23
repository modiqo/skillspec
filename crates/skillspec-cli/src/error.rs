use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse YAML from {path}: {source}")]
    ParseYaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("failed to parse JSON from {path}: {source}")]
    ParseJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to render YAML for {path}: {source}")]
    RenderYaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("failed to render JSON: {0}")]
    RenderJson(#[from] serde_json::Error),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("failed to write output: {0}")]
    Output(#[from] std::io::Error),
    #[error("unsupported schema {found:?}; expected skillspec/v0")]
    UnsupportedSchema { found: String },
    #[error("spec is missing required field {field}")]
    MissingField { field: &'static str },
    #[error("invalid identifier {value:?} in {field}")]
    InvalidIdentifier { field: &'static str, value: String },
    #[error("duplicate identifier {value:?} in {field}")]
    DuplicateId { field: &'static str, value: String },
    #[error("unknown reference {value:?} in {field}")]
    UnknownReference { field: &'static str, value: String },
    #[error("{message}")]
    InvalidInput { message: String },
}
