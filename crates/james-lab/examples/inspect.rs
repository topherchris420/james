//! Read-only live check: cargo run -p james-lab --example inspect -- 5173
use james_lab::LabClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "5173".into())
        .parse()?;
    let client = LabClient::new(port)?;
    let status = client.status()?;
    println!(
        "Runtime configured={}, reachable={:?}, failure={:?}",
        status.configured, status.reachable, status.failure
    );
    let context = client.context()?;
    println!(
        "Discovery available={}, active={}, stage={}, history={}",
        context.available,
        context.active,
        context.stage,
        context.history.len()
    );
    println!("Question: {}", context.question);
    Ok(())
}
