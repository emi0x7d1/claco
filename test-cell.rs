#[tokio::main]
async fn main() {
    let cell = std::sync::Arc::new(tokio::sync::OnceCell::<String>::new());
    if let Ok(val) = cell.get_or_try_init(|| async { Result::<_, ()>::Ok("hi".to_string()) }).await {
        println!("{}", val);
    }
}
