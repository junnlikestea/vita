use async_std::prelude::*;
extern crate vita;
use self::vita::*;
use sources::bufferover;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let bufferover = bufferover::run("hackerone.com").await?;
    for s in bufferover {
        println!("{}", s);
    }
    Ok(())
}
