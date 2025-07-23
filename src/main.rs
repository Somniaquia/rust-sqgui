use sq::App;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut app = App::new().await?;
    
    app.create_window("sq", 2560, 1440).await?;
    app.run().await;
    Ok(())
}