#![allow(unused, unused_variables)]
use sq::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut app = SQ::new().await?;

    app.create_window("sq", 800, 600).await?;
    app.run().await?;
    Ok(())
}
