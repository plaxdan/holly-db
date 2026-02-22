mod commands;

use clap::{Parser, Subcommand};
use holly_core::{ContextFormat, HollyDb};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "holly",
    about = "Portable knowledge graph for developers and their agents",
    version
)]
struct Cli {
    /// Path to the holly database
    #[arg(long, env = "HOLLY_DB_PATH", global = true)]
    db: Option<PathBuf>,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize holly-db in the current directory
    Init {
        /// Initialize in the global home directory
        #[arg(long)]
        global: bool,
        /// Download the semantic search model
        #[arg(long)]
        download_model: bool,
    },

    /// Quick capture — store a memory node
    #[command(alias = "mem")]
    Remember { text: String },

    /// Record a structured knowledge node
    #[command(alias = "r")]
    Record {
        /// Node type (decision, constraint, error, idea, etc.)
        #[arg(short = 't', long)]
        r#type: String,

        /// Node title
        title: String,

        /// Content as JSON
        #[arg(short, long)]
        content: Option<String>,

        /// Repository name
        #[arg(long)]
        repo: Option<String>,

        /// Initial status
        #[arg(long)]
        status: Option<String>,

        /// Source (curated or auto)
        #[arg(long)]
        source: Option<String>,

        /// Tags
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
    },

    /// Search for nodes
    #[command(alias = "s")]
    Search {
        query: String,

        #[arg(short = 't', long)]
        r#type: Option<String>,

        #[arg(long)]
        repo: Option<String>,

        #[arg(long)]
        status: Option<String>,

        /// Enable semantic (hybrid) search
        #[arg(long)]
        semantic: bool,

        #[arg(short, long, default_value = "20")]
        limit: u32,
    },

    /// List nodes
    #[command(alias = "l")]
    List {
        #[arg(short = 't', long)]
        r#type: Option<String>,

        #[arg(long)]
        repo: Option<String>,

        #[arg(long)]
        status: Option<String>,

        #[arg(long)]
        source: Option<String>,

        #[arg(short, long, default_value = "50")]
        limit: u32,
    },

    /// Get a node by ID
    #[command(alias = "g")]
    Get {
        id: String,

        /// Also show related edges
        #[arg(long)]
        related: bool,
    },

    /// Update a node
    #[command(alias = "update")]
    Edit {
        id: String,

        #[arg(long)]
        title: Option<String>,

        #[arg(long)]
        content: Option<String>,

        /// Replace content instead of merging
        #[arg(long)]
        replace: bool,

        #[arg(long)]
        status: Option<String>,

        #[arg(long)]
        repo: Option<String>,

        #[arg(long, value_delimiter = ',')]
        tags: Option<Vec<String>>,
    },

    /// Delete a node
    #[command(alias = "rm")]
    Delete {
        id: String,

        #[arg(long)]
        force: bool,
    },

    /// Connect two nodes with an edge
    Connect {
        from: String,
        to: String,

        #[arg(short = 't', long, default_value = "relates_to")]
        r#type: String,
    },

    /// Export context for agents
    Context {
        /// Output format
        #[arg(long, default_value = "markdown")]
        format: String,

        /// Output file (default: stdout)
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Audit database health
    Audit {
        /// Fix safe issues automatically
        #[arg(long)]
        fix: bool,

        /// Stale threshold in days
        #[arg(long, default_value = "14")]
        stale_days: u32,
    },

    /// Show statistics
    Stats,

    /// Event operations
    Event {
        #[command(subcommand)]
        action: EventAction,
    },

    /// Task operations
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Run operations
    Run {
        #[command(subcommand)]
        action: RunAction,
    },

    /// Add or remove tags on a node
    Tag {
        id: String,

        /// Tags to add (or remove with --remove)
        tags: Vec<String>,

        /// Remove the listed tags instead of adding them
        #[arg(long)]
        remove: bool,
    },

    /// Check for a newer version of holly
    #[command(name = "update-check")]
    UpdateCheck {
        /// Print current version even if up to date
        #[arg(long)]
        verbose: bool,
    },

    /// Import from a legacy Holly database
    Import {
        /// Path to legacy holly.db
        #[arg(long)]
        from: PathBuf,
    },

    /// Regenerate vector embeddings for nodes missing from the index
    Reindex,

    /// Run the MCP server (stdio transport)
    #[command(name = "mcp-server")]
    McpServer,

    /// Manage MCP configuration
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
}

#[derive(Subcommand)]
enum EventAction {
    /// Record an event
    Record {
        event_type: String,

        #[arg(long)]
        payload: Option<String>,

        #[arg(long)]
        repo: Option<String>,

        #[arg(long)]
        workspace: Option<String>,

        #[arg(long)]
        idempotency_key: Option<String>,
    },

    /// List events
    List {
        #[arg(short = 't', long)]
        r#type: Option<String>,

        #[arg(long)]
        repo: Option<String>,

        #[arg(long)]
        workspace: Option<String>,

        #[arg(short, long, default_value = "50")]
        limit: u32,
    },
}

#[derive(Subcommand)]
enum TaskAction {
    /// Create a new task
    Create {
        title: String,

        #[arg(long)]
        repo: Option<String>,

        #[arg(long)]
        priority: Option<String>,
    },
    /// Start a task (planned → in_progress)
    Start { id: String },
    /// Complete a task
    Complete { id: String },
    /// Block a task
    Block { id: String },
    /// Cancel a task
    Cancel { id: String },
    /// List tasks
    List {
        #[arg(long)]
        status: Option<String>,
    },
}

#[derive(Subcommand)]
enum McpAction {
    /// Add holly-db to .mcp.json in the current directory
    Enable,
    /// Remove holly-db from .mcp.json
    Disable,
    /// Show MCP configuration status
    Status,
}

#[derive(Subcommand)]
enum RunAction {
    /// Start a run linked to a task
    Start {
        #[arg(long)]
        task: String,

        title: Option<String>,
    },
    /// Complete a run
    Complete {
        id: String,

        #[arg(long)]
        status: Option<String>,

        #[arg(long)]
        summary: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let json = cli.json;

    // `init` doesn't need an existing DB
    if let Commands::Init {
        global,
        download_model,
    } = &cli.command
    {
        if *download_model {
            return commands::init::download_model();
        }
        return commands::init::run(*global, cli.db.as_deref());
    }

    // `update-check` — no DB needed
    if let Commands::UpdateCheck { verbose } = &cli.command {
        return commands::update_check::run(*verbose);
    }

    // `mcp-server` opens its own DB inside the async runtime
    if let Commands::McpServer = &cli.command {
        return commands::mcp::run_server();
    }

    // `mcp enable/disable/status` — resolves DB path but doesn't open it
    if let Commands::Mcp { action } = &cli.command {
        let db_path = HollyDb::resolve_path(cli.db.as_deref());
        return match action {
            McpAction::Enable => commands::mcp::enable(&db_path),
            McpAction::Disable => commands::mcp::disable(),
            McpAction::Status => commands::mcp::status(),
        };
    }

    // All other commands require an open DB
    let db_path = HollyDb::resolve_path(cli.db.as_deref());
    let db = HollyDb::open(&db_path)?;

    match cli.command {
        Commands::Init { .. } => unreachable!(),

        Commands::Remember { text } => {
            commands::remember::run(&db, &text, json)?;
        }

        Commands::Record {
            r#type,
            title,
            content,
            repo,
            status,
            source,
            tags,
        } => {
            commands::record::run(
                &db,
                &r#type,
                &title,
                content.as_deref(),
                repo.as_deref(),
                status.as_deref(),
                source.as_deref(),
                tags,
                json,
            )?;
        }

        Commands::Search {
            query,
            r#type,
            repo,
            status,
            semantic,
            limit,
        } => {
            commands::search::run(
                &db,
                &query,
                r#type.as_deref(),
                repo.as_deref(),
                status.as_deref(),
                semantic,
                limit,
                json,
            )?;
        }

        Commands::List {
            r#type,
            repo,
            status,
            source,
            limit,
        } => {
            commands::list::run(
                &db,
                r#type.as_deref(),
                repo.as_deref(),
                status.as_deref(),
                source.as_deref(),
                limit,
                json,
            )?;
        }

        Commands::Get { id, related } => {
            commands::get::run(&db, &id, related, json)?;
        }

        Commands::Edit {
            id,
            title,
            content,
            replace,
            status,
            repo,
            tags,
        } => {
            commands::edit::run(
                &db,
                &id,
                title.as_deref(),
                content.as_deref(),
                replace,
                status.as_deref(),
                repo.as_deref(),
                tags,
                json,
            )?;
        }

        Commands::Delete { id, force } => {
            commands::delete::run(&db, &id, force, json)?;
        }

        Commands::Connect { from, to, r#type } => {
            commands::connect::run(&db, &from, &to, &r#type, json)?;
        }

        Commands::Context { format, output } => {
            let fmt = if format == "json" {
                ContextFormat::Json
            } else {
                ContextFormat::Markdown
            };
            commands::context::run(&db, fmt, output.as_deref(), json)?;
        }

        Commands::Audit { fix, stale_days } => {
            commands::audit::run(&db, fix, stale_days, json)?;
        }

        Commands::Stats => {
            commands::stats::run(&db, json)?;
        }

        Commands::Event { action } => match action {
            EventAction::Record {
                event_type,
                payload,
                repo,
                workspace,
                idempotency_key,
            } => {
                commands::event::record(
                    &db,
                    &event_type,
                    payload.as_deref(),
                    repo.as_deref(),
                    workspace.as_deref(),
                    idempotency_key.as_deref(),
                    json,
                )?;
            }
            EventAction::List {
                r#type,
                repo,
                workspace,
                limit,
            } => {
                commands::event::list(
                    &db,
                    r#type.as_deref(),
                    repo.as_deref(),
                    workspace.as_deref(),
                    limit,
                    json,
                )?;
            }
        },

        Commands::Task { action } => match action {
            TaskAction::Create {
                title,
                repo,
                priority,
            } => {
                commands::task::create(&db, &title, repo.as_deref(), priority.as_deref(), json)?;
            }
            TaskAction::Start { id } => {
                commands::task::start(&db, &id, json)?;
            }
            TaskAction::Complete { id } => {
                commands::task::complete(&db, &id, json)?;
            }
            TaskAction::Block { id } => {
                commands::task::block(&db, &id, json)?;
            }
            TaskAction::Cancel { id } => {
                commands::task::cancel(&db, &id, json)?;
            }
            TaskAction::List { status } => {
                commands::task::list(&db, status.as_deref(), json)?;
            }
        },

        Commands::Run { action } => match action {
            RunAction::Start { task, title } => {
                commands::run::start(&db, &task, title.as_deref(), json)?;
            }
            RunAction::Complete {
                id,
                status,
                summary,
            } => {
                commands::run::complete(&db, &id, status.as_deref(), summary.as_deref(), json)?;
            }
        },

        Commands::Tag { id, tags, remove } => {
            commands::tag::run(&db, &id, tags, remove, json)?;
        }

        Commands::Import { from } => {
            commands::import::run(&db, &from, json)?;
        }

        Commands::Reindex => {
            commands::reindex::run(&db, json)?;
        }

        Commands::UpdateCheck { .. } => unreachable!("handled above"),
        Commands::McpServer => unreachable!("handled above"),
        Commands::Mcp { .. } => unreachable!("handled above"),
    }

    Ok(())
}
