use crate::tools::{edges, events, maintenance, nodes, search, tasks};
use holly_core::HollyDb;
use rmcp::{
    Error as McpError, ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, Content, Implementation, ListToolsResult,
        PaginatedRequestParam, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
};
use serde_json::{Map, Value, json};
use std::sync::{Arc, Mutex};

type Db = Arc<Mutex<HollyDb>>;

#[derive(Clone)]
pub struct HollyServer {
    db: Db,
}

impl HollyServer {
    pub fn new(db: HollyDb) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
        }
    }

    fn tools_list() -> Vec<Tool> {
        let schema = |props: Value| {
            let obj = json!({
                "type": "object",
                "properties": props,
            });
            Arc::new(obj.as_object().unwrap().clone())
        };

        vec![
            Tool::new(
                "holly_record",
                "Record a new node in the knowledge graph with automatic embedding.",
                schema(json!({
                    "node_type": {"type": "string", "description": "One of: idea, goal, requirement, decision, implementation, error, improvement, constraint, task, run, artifact, defect, override, policy"},
                    "title": {"type": "string", "description": "Short descriptive title"},
                    "content": {"type": "string", "description": "Free-form text (parsed into structured fields based on node_type)"},
                    "repo": {"type": "string", "description": "Repository name this knowledge pertains to"},
                    "source": {"type": "string", "description": "'curated' (default) or 'auto'"},
                    "domain": {"type": "string", "description": "Override domain"},
                })),
            ),
            Tool::new(
                "holly_get",
                "Get full details of a node including its relationships.",
                schema(json!({
                    "id": {"type": "string", "description": "UUID of the node to retrieve"},
                })),
            ),
            Tool::new(
                "holly_list",
                "List nodes in the knowledge graph, optionally filtered by type.",
                schema(json!({
                    "node_type": {"type": "string", "description": "Filter by type"},
                    "repo": {"type": "string", "description": "Filter by repo"},
                    "status": {"type": "string", "description": "Filter by status"},
                    "source": {"type": "string", "description": "Filter by source ('auto' or 'curated')"},
                    "limit": {"type": "integer", "description": "Max results (default 20)"},
                })),
            ),
            Tool::new(
                "holly_recent",
                "List recent nodes as a markdown table. Returns nodes from the last N days.",
                schema(json!({
                    "days": {"type": "integer", "description": "Days to look back (default 7)"},
                    "limit": {"type": "integer", "description": "Max results (default 20)"},
                })),
            ),
            Tool::new(
                "holly_update",
                "Update an existing node. Content updates are merge-safe by default.",
                schema(json!({
                    "id": {"type": "string", "description": "UUID of the node to update"},
                    "title": {"type": "string", "description": "New title (leave empty to keep current)"},
                    "content": {"type": "string", "description": "New content text (leave empty to keep current)"},
                    "replace_content": {"type": "boolean", "description": "If true, replaces content instead of merging"},
                    "repo": {"type": "string", "description": "New repo value"},
                    "status": {"type": "string", "description": "New status"},
                })),
            ),
            Tool::new(
                "holly_delete",
                "Delete a node and all its edges from the knowledge graph.",
                schema(json!({
                    "id": {"type": "string", "description": "UUID of the node to delete"},
                })),
            ),
            Tool::new(
                "holly_related",
                "Find nodes semantically similar to a given node.",
                schema(json!({
                    "id": {"type": "string", "description": "UUID of the reference node"},
                    "limit": {"type": "integer", "description": "Max results (default 10)"},
                    "node_type": {"type": "string", "description": "Filter results by type"},
                })),
            ),
            Tool::new(
                "holly_search",
                "Semantic similarity search across the knowledge graph.",
                schema(json!({
                    "query": {"type": "string", "description": "Natural language search query"},
                    "node_type": {"type": "string", "description": "Filter by type"},
                    "repo": {"type": "string", "description": "Repository name this knowledge pertains to"},
                    "source": {"type": "string", "description": "Filter by source ('auto' or 'curated')"},
                    "status": {"type": "string", "description": "Filter by status"},
                    "limit": {"type": "integer", "description": "Max results (default 10)"},
                })),
            ),
            Tool::new(
                "holly_text_search",
                "Keyword/full-text search across the knowledge graph.",
                schema(json!({
                    "query": {"type": "string", "description": "Keywords to search for"},
                    "node_type": {"type": "string", "description": "Filter by type"},
                    "repo": {"type": "string", "description": "Filter by repo"},
                    "source": {"type": "string", "description": "Filter by source"},
                    "status": {"type": "string", "description": "Filter by status"},
                    "limit": {"type": "integer", "description": "Max results (default 10)"},
                })),
            ),
            Tool::new(
                "holly_connect",
                "Create a relationship (edge) between two nodes.",
                schema(json!({
                    "from_id": {"type": "string", "description": "UUID of the source node"},
                    "to_id": {"type": "string", "description": "UUID of the target node"},
                    "edge_type": {"type": "string", "description": "One of: derives_from, implements, blocks, caused_by, fixes, relates_to, supersedes"},
                })),
            ),
            Tool::new(
                "holly_delete_orphaned_edges",
                "Delete edges that reference non-existent nodes.",
                schema(json!({})),
            ),
            Tool::new(
                "holly_event_record",
                "Record a lifecycle event (workspace created, skill invoked, PR pushed, etc.).",
                schema(json!({
                    "event_type": {"type": "string", "description": "Event type (e.g., workspace_created, skill_invoked, pr_pushed)"},
                    "payload": {"type": "string", "description": "JSON payload with event details"},
                    "repo": {"type": "string", "description": "Repository name this event pertains to"},
                    "workspace": {"type": "string", "description": "Workspace name, if applicable"},
                })),
            ),
            Tool::new(
                "holly_event_list",
                "List lifecycle events, optionally filtered by type or workspace.",
                schema(json!({
                    "event_type": {"type": "string", "description": "Filter by event type"},
                    "repo": {"type": "string", "description": "Repository name to filter events by"},
                    "workspace": {"type": "string", "description": "Filter by workspace name"},
                    "limit": {"type": "integer", "description": "Max results (default 50)"},
                })),
            ),
            Tool::new(
                "holly_task_create",
                "Create a task node with task-specific fields and status governance.",
                schema(json!({
                    "title": {"type": "string", "description": "Task title"},
                    "description": {"type": "string", "description": "Optional free-form task description"},
                    "repo": {"type": "string", "description": "Repository this task belongs to"},
                    "source": {"type": "string", "description": "'curated' (default) or 'auto'"},
                    "status": {"type": "string", "description": "Task status: planned, in_progress, blocked, completed, cancelled"},
                    "priority": {"type": "string", "description": "Priority (default: medium)"},
                    "owner": {"type": "string", "description": "Task owner"},
                    "depends_on": {"type": "array", "items": {"type": "string"}, "description": "Optional list of task node IDs this task depends on"},
                    "evidence": {"type": "array", "items": {"type": "string"}, "description": "Optional initial evidence references"},
                })),
            ),
            Tool::new(
                "holly_task_transition",
                "Transition a task status with transition validation and event recording.",
                schema(json!({
                    "id": {"type": "string", "description": "Task node UUID"},
                    "status": {"type": "string", "description": "New status: planned, in_progress, blocked, completed, cancelled"},
                    "note": {"type": "string", "description": "Optional transition note"},
                })),
            ),
            Tool::new(
                "holly_task_list",
                "List task nodes with optional status/source/repo filters.",
                schema(json!({
                    "status": {"type": "string", "description": "Optional status filter (planned, in_progress, blocked, completed, cancelled)"},
                    "repo": {"type": "string", "description": "Optional repo filter"},
                    "source": {"type": "string", "description": "Optional source filter"},
                    "limit": {"type": "integer", "description": "Max results (default 20)"},
                })),
            ),
            Tool::new(
                "holly_run_start",
                "Start a run linked to a task and optionally move the task to in_progress.",
                schema(json!({
                    "task_id": {"type": "string", "description": "Task node UUID"},
                    "title": {"type": "string", "description": "Optional run title"},
                    "repo": {"type": "string", "description": "Optional repo override"},
                    "workspace": {"type": "string", "description": "Optional workspace context"},
                    "source": {"type": "string", "description": "'curated' (default) or 'auto'"},
                })),
            ),
            Tool::new(
                "holly_run_complete",
                "Complete a run with final status (completed, failed, or aborted).",
                schema(json!({
                    "run_id": {"type": "string", "description": "Run node UUID"},
                    "status": {"type": "string", "description": "Final status: completed, failed, aborted"},
                    "summary": {"type": "string", "description": "Optional completion summary"},
                    "artifacts": {"type": "array", "items": {"type": "string"}, "description": "Optional artifact IDs or paths"},
                })),
            ),
            Tool::new(
                "holly_task_link_artifact",
                "Create an artifact node and link it to a task and optional run.",
                schema(json!({
                    "task_id": {"type": "string", "description": "Task node UUID"},
                    "title": {"type": "string", "description": "Artifact title"},
                    "path": {"type": "string", "description": "Artifact path or locator"},
                    "run_id": {"type": "string", "description": "Optional run node UUID"},
                    "artifact_type": {"type": "string", "description": "Artifact type (default: evidence)"},
                    "notes": {"type": "string", "description": "Optional notes"},
                    "repo": {"type": "string", "description": "Optional repo"},
                    "source": {"type": "string", "description": "'curated' (default) or 'auto'"},
                })),
            ),
            Tool::new(
                "holly_audit",
                "Audit the Holly knowledge graph for health issues.",
                schema(json!({
                    "stale_days": {"type": "integer", "description": "Days without update before a node is considered stale (default 14)"},
                    "mode": {"type": "string", "description": "Output mode: 'summary' for issue counts, 'detail' for per-issue descriptions"},
                    "similarity_threshold": {"type": "number", "description": "Minimum similarity score for missing-edge suggestions (default 0.85)"},
                    "duplicate_threshold": {"type": "number", "description": "Minimum similarity score for potential duplicate detection (default 0.92)"},
                })),
            ),
            Tool::new(
                "holly_stats",
                "Work pattern analytics from the knowledge graph.",
                schema(json!({
                    "days": {"type": "integer", "description": "Days to look back (0 = all time, default 30)"},
                })),
            ),
        ]
    }
}

impl ServerHandler for HollyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities {
                tools: Some(Default::default()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "holly-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Holly knowledge graph MCP server. Use holly_record to capture knowledge, holly_search to find it.".to_string()),
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            tools: Self::tools_list(),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let args: Map<String, Value> = request.arguments.unwrap_or_default();
        let db = self.db.clone();

        let result = match request.name.as_ref() {
            "holly_record" => nodes::holly_record(db, args).await,
            "holly_get" => nodes::holly_get(db, args).await,
            "holly_list" => nodes::holly_list(db, args).await,
            "holly_recent" => nodes::holly_recent(db, args).await,
            "holly_update" => nodes::holly_update(db, args).await,
            "holly_delete" => nodes::holly_delete(db, args).await,
            "holly_related" => nodes::holly_related(db, args).await,
            "holly_search" => search::holly_search(db, args).await,
            "holly_text_search" => search::holly_text_search(db, args).await,
            "holly_connect" => edges::holly_connect(db, args).await,
            "holly_delete_orphaned_edges" => edges::holly_delete_orphaned_edges(db, args).await,
            "holly_event_record" => events::holly_event_record(db, args).await,
            "holly_event_list" => events::holly_event_list(db, args).await,
            "holly_task_create" => tasks::holly_task_create(db, args).await,
            "holly_task_transition" => tasks::holly_task_transition(db, args).await,
            "holly_task_list" => tasks::holly_task_list(db, args).await,
            "holly_run_start" => tasks::holly_run_start(db, args).await,
            "holly_run_complete" => tasks::holly_run_complete(db, args).await,
            "holly_task_link_artifact" => tasks::holly_task_link_artifact(db, args).await,
            "holly_audit" => maintenance::holly_audit(db, args).await,
            "holly_stats" => maintenance::holly_stats(db, args).await,
            _ => CallToolResult::error(vec![Content::text(format!(
                "Unknown tool: {}",
                request.name
            ))]),
        };

        Ok(result)
    }
}

/// Entry point for `holly mcp-server`.
pub async fn run_server() -> anyhow::Result<()> {
    let db_path = HollyDb::resolve_path(None);
    let db = HollyDb::open(&db_path)?;
    let server = HollyServer::new(db);

    use rmcp::ServiceExt;
    let transport = rmcp::transport::stdio();
    let running = server.serve(transport).await?;
    running.waiting().await?;
    Ok(())
}
