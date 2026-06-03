use anyhow::Result;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    println!("pulse-agent starting...");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
