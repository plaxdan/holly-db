pub mod audit;
pub mod config;
pub mod context;
pub mod db;
pub mod edges;
pub mod embeddings;
pub mod error;
pub mod events;
pub mod import;
pub mod nodes;
pub mod provenance;
pub mod schema;
pub mod search;
pub mod stats;
pub mod types;

// Convenience re-exports
pub use audit::AuditReport;
pub use config::HollyConfig;
pub use context::{ContextExport, ContextFormat};
pub use db::HollyDb;
pub use edges::Edge;
pub use error::{HollyError, Result};
pub use events::{HollyEvent, ListEventsFilter};
pub use import::ImportStats;
pub use nodes::{
    embedding_text, CreateNodeInput, ListNodesFilter, Node, ReindexStats, UpdateNodeInput,
};
pub use provenance::Provenance;
pub use search::{SearchOptions, SearchResult};
pub use stats::Stats;
