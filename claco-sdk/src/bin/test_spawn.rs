use claco_termulator::SessionOptions;

#[tokio::main]
async fn main() {
    let options = SessionOptions {
        program: "claude".to_string(),
        args: vec!["--dangerously-skip-permissions".to_string()],
        cols: 120,
        rows: 40,
        cwd: None,
    };
    let session = claco_termulator::Session::spawn(options).unwrap();

    let mut accepted = false;
    for _ in 0..10 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if let Ok(text) = session.get_text().await {
            if !accepted && text.contains("Yes, I accept") {
                accepted = true;
                session.input().down().enter();
            }
            println!("== TERM ==\n{}\n==========", text);
        }
    }
}
