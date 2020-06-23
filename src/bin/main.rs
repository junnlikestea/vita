use async_std::prelude::*;
extern crate vita;
use self::vita::*;
use sources::hackertarget::run;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let bufferover = run("anVubmxpa2V0ZWE.com").await?;
    for r in bufferover {
        println!("{}", r);
    }
    Ok(())
}
