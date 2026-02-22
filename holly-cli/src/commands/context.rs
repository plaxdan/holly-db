use holly_core::{ContextFormat, HollyDb};
use std::path::Path;

pub fn run(
    db: &HollyDb,
    format: ContextFormat,
    output: Option<&Path>,
    json: bool,
) -> anyhow::Result<()> {
    let content = if json || format == ContextFormat::Json {
        let ctx = db.export_context()?;
        serde_json::to_string_pretty(&ctx)?
    } else {
        db.export_context_markdown()?
    };

    if let Some(path) = output {
        std::fs::write(path, &content)?;
        eprintln!("Context written to {}", path.display());
    } else {
        print!("{}", content);
    }
    Ok(())
}
