use async_std::prelude::*;
extern crate vita;
use self::vita::*;
use sources::anubisdb::run;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let urls: Vec<String> = vec!["hackerone.com".to_string(), "example.com".to_string()];
    let ress = vita::runner(urls).await;
    ress.into_iter()
        .flat_map(|v| v.into_iter())
        .map(|s| println!("{}", s))
        .collect::<Vec<_>>();
    Ok(())
}
