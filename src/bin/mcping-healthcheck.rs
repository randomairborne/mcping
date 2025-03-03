#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(url) = std::env::args().nth(1) else {
        return Err("`mcping-healthcheck` requires exactly one argument.".into());
    };
    if std::env::args().len() != 2 {
        return Err("`mcping-healthcheck` requires exactly one argument.".into());
    }
    reqwest::get(url).await?.error_for_status()?;
    println!("Health check succeeded");
    Ok(())
}