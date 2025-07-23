use sq::App;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut app = App::new().await?;
    
    app.create_window("sq", 800, 600).await?;
    app.run().await;
    Ok(())
}