use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("pulse-master starting...");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
