use holly_core::{CreateNodeInput, HollyDb};
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn link(
    db: &HollyDb,
    task_id: &str,
    path: &str,
    title: Option<&str>,
    run_id: Option<&str>,
    notes: Option<&str>,
    artifact_type: &str,
    json: bool,
) -> anyhow::Result<()> {
    // Verify task exists (also resolves 8-char prefix to full UUID)
    let task = db.get_node(task_id)?;

    // Default title to the filename component of path
    let default_title;
    let resolved_title = match title {
        Some(t) => t,
        None => {
            default_title = Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path)
                .to_string();
            &default_title
        }
    };

    let mut content = serde_json::json!({
        "status": "recorded",
        "artifact_type": artifact_type,
        "path": path,
        "task_id": task.id,
        "run_id": run_id.unwrap_or(""),
    });

    if let Some(n) = notes {
        content["notes"] = serde_json::Value::String(n.to_string());
    }

    let artifact = db.create_node(CreateNodeInput {
        node_type: "artifact".to_string(),
        title: resolved_title.to_string(),
        content: Some(content),
        status: Some("recorded".to_string()),
        ..Default::default()
    })?;

    // Link artifact → task
    let _ = db.create_edge(&artifact.id, &task.id, "relates_to", None);

    // Link artifact → run if provided
    if let Some(rid) = run_id {
        if !rid.is_empty() {
            // Verify run exists before linking (best-effort; ignore lookup errors)
            if let Ok(run) = db.get_node(rid) {
                let _ = db.create_edge(&artifact.id, &run.id, "relates_to", None);
            }
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&artifact)?);
    } else {
        println!(
            "Artifact linked [{}]: {} → task [{}]",
            &artifact.id[..8],
            artifact.title,
            &task.id[..8],
        );
    }
    Ok(())
}
