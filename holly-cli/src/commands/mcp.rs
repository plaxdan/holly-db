pub fn run_server() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(holly_mcp::run_server())
}
