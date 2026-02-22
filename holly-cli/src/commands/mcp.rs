use anyhow::Context;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub fn run_server() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(holly_mcp::run_server())
}

/// Add holly-db to .mcp.json in the current directory.
pub fn enable(db_path: &Path) -> anyhow::Result<()> {
    let mcp_path = mcp_json_path();
    let mut mcp: Value = if mcp_path.exists() {
        let content = std::fs::read_to_string(&mcp_path).context("Failed to read .mcp.json")?;
        serde_json::from_str(&content).context("Failed to parse .mcp.json — is it valid JSON?")?
    } else {
        json!({ "mcpServers": {} })
    };

    // Ensure mcpServers key exists
    if mcp.get("mcpServers").is_none() {
        mcp["mcpServers"] = json!({});
    }

    let servers = mcp["mcpServers"]
        .as_object_mut()
        .context(".mcp.json has unexpected structure (expected {\"mcpServers\": {...}})")?;

    let db_path_str = db_path
        .to_str()
        .context("DB path contains non-UTF-8 characters")?;

    servers.insert(
        "holly-db".to_string(),
        json!({
            "command": "holly",
            "args": ["mcp-server"],
            "env": {
                "HOLLY_DB_PATH": db_path_str
            }
        }),
    );

    let content = serde_json::to_string_pretty(&mcp)?;
    std::fs::write(&mcp_path, content)?;

    println!("holly-db added to {}", mcp_path.display());
    println!();
    println!("DB path: {}", db_path_str);
    println!();
    println!("Restart your MCP client (Claude Code, Cursor, etc.) to pick up the change.");
    Ok(())
}

/// Remove holly-db from .mcp.json.
pub fn disable() -> anyhow::Result<()> {
    let mcp_path = mcp_json_path();

    if !mcp_path.exists() {
        println!("No .mcp.json found in current directory — nothing to disable.");
        return Ok(());
    }

    let content = std::fs::read_to_string(&mcp_path).context("Failed to read .mcp.json")?;
    let mut mcp: Value =
        serde_json::from_str(&content).context("Failed to parse .mcp.json — is it valid JSON?")?;

    let removed = mcp
        .get_mut("mcpServers")
        .and_then(|s| s.as_object_mut())
        .map(|s| s.remove("holly-db").is_some())
        .unwrap_or(false);

    if removed {
        let content = serde_json::to_string_pretty(&mcp)?;
        std::fs::write(&mcp_path, content)?;
        println!("holly-db removed from {}", mcp_path.display());
        println!("Restart your MCP client to pick up the change.");
    } else {
        println!(
            "holly-db was not found in {} — nothing to remove.",
            mcp_path.display()
        );
    }

    Ok(())
}

/// Show current MCP configuration status.
pub fn status() -> anyhow::Result<()> {
    let mcp_path = mcp_json_path();

    // Check for holly binary on PATH
    let holly_bin = which_holly();

    // Check .mcp.json
    let (mcp_found, holly_entry) = if mcp_path.exists() {
        let content = std::fs::read_to_string(&mcp_path).context("Failed to read .mcp.json")?;
        match serde_json::from_str::<Value>(&content) {
            Ok(mcp) => {
                let entry = mcp
                    .get("mcpServers")
                    .and_then(|s| s.get("holly-db"))
                    .cloned();
                (true, entry)
            }
            Err(_) => (true, None),
        }
    } else {
        (false, None)
    };

    println!("MCP status");
    println!();

    match &holly_bin {
        Some(path) => println!("  holly binary : found ({})", path.display()),
        None => println!("  holly binary : not found on PATH"),
    }

    if mcp_found {
        println!("  .mcp.json    : found ({})", mcp_path.display());
    } else {
        println!("  .mcp.json    : not found");
    }

    match &holly_entry {
        Some(entry) => {
            println!("  holly-db     : configured");
            if let Some(db_path) = entry.get("env").and_then(|e| e.get("HOLLY_DB_PATH")) {
                println!("  db path      : {}", db_path.as_str().unwrap_or("?"));
            }
        }
        None => {
            println!("  holly-db     : not configured");
            if holly_bin.is_some() {
                println!();
                println!("Run 'holly mcp enable' to add holly-db to .mcp.json.");
            } else {
                println!();
                println!("Install holly first, then run 'holly mcp enable'.");
            }
        }
    }

    Ok(())
}

fn mcp_json_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".mcp.json")
}

fn which_holly() -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join("holly");
            if candidate.is_file() {
                Some(candidate)
            } else {
                None
            }
        })
    })
}
