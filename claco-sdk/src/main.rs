use claco_sdk::ClacoSession;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let session = ClacoSession::spawn(None).await?;

    println!("Sending prompt...");
    let response_stream = session
        .send("Write hello world to a markdown file.")
        .await?;
    tokio::pin!(response_stream);

    while let Some(response) = response_stream.next().await {
        match response {
            claco_sdk::ClacoResponse::Text(text) => {
                println!("\n--- CLACO TEXT ---\n{}", text);
            }
            claco_sdk::ClacoResponse::ToolCall { name, args } => {
                println!("\n--- CLACO TOOL CALL: {}({}) ---", name, args);
            }
        }
    }

    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("\nResponse finished.");
    Ok(())
}
