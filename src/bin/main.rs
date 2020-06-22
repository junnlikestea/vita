use async_std::prelude::*;
extern crate vita;
use self::vita::*;
use sources::certspotter;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let bufferover = certspotter::run("hackerone.com").await?;
    Ok(())
}
