use anyhow::Result;
use pulse_master::service::PulseService;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "[::1]:50051".parse()?;
    println!("pulse-master listening on {addr}");

    Server::builder().add_service(PulseService::new().into_server()).serve(addr).await?;

    Ok(())
}
