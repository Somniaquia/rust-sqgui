use sq::App;

#[tokio::main]
async fn main() {
    let app = App::new().await;
    app.run().await;
}