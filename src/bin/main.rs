use async_std::prelude::*;
extern crate vita;
use self::vita::*;
use sources::anubisdb::run;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let bufferover = run("hackerone.com").await?;
    for r in bufferover {
        println!("{}", r);
    }
    Ok(())
}
