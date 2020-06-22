extern crate vita;
use futures_await_test::async_test;
use vita::sources::bufferover::*;

// Checks if the run method is returning results from the api.
#[async_test]
async fn bufferover_results() {
    let results = run("hackerone.com").await.unwrap();
    assert!(results.len() > 10);
}
