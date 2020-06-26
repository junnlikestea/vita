extern crate vita;
use self::vita::*;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let hosts: Vec<String> = vec!["starbucks.com".to_string()];
    let results = vita::runner(hosts).await;
    for r in results {
        println!("{}", r);
    }
    Ok(())
}
