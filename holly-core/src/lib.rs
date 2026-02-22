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
pub use db::HollyDb;
pub use error::{HollyError, Result};
pub use nodes::{CreateNodeInput, ListNodesFilter, Node, ReindexStats, UpdateNodeInput, embedding_text};
pub use edges::Edge;
pub use events::{HollyEvent, ListEventsFilter};
pub use search::{SearchOptions, SearchResult};
pub use provenance::Provenance;
pub use audit::AuditReport;
pub use stats::Stats;
pub use context::{ContextExport, ContextFormat};
pub use import::ImportStats;
pub use config::HollyConfig;
