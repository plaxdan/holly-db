use thiserror::Error;

#[derive(Debug, Error)]
pub enum HollyError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("node not found: {0}")]
    NodeNotFound(String),

    #[error("edge not found: {from} -> {to} ({edge_type})")]
    EdgeNotFound {
        from: String,
        to: String,
        edge_type: String,
    },

    #[error("invalid status '{status}' for type '{node_type}'. Allowed: {allowed}")]
    InvalidStatus {
        status: String,
        node_type: String,
        allowed: String,
    },

    #[error("invalid transition from '{from}' to '{to}' for type '{node_type}'")]
    InvalidTransition {
        from: String,
        to: String,
        node_type: String,
    },

    #[error("invalid node type: {0}")]
    InvalidNodeType(String),

    #[error("invalid edge type: {0}")]
    InvalidEdgeType(String),

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("import error: {0}")]
    Import(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, HollyError>;
