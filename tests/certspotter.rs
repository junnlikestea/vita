extern crate vita;
use futures_await_test::async_test;
use vita::sources::certspotter::*;

#[async_test]
async fn certspotter_results() {
    let results = run("hackerone.com").await.unwrap();
    assert!(results.len() > 5);
}
