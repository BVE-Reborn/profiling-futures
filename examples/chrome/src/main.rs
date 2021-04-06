use std::{path::Path, time::Duration};

fn main() {
    wgpu_subscriber::initialize_default_subscriber(Some(Path::new("chrome.json")));

    let handle = async_std::task::spawn(some_work());

    async_std::task::block_on(handle);
}

#[profiling_futures::async_instrument]
async fn some_work() {
    profiling_futures::enter!("span");
    profiling_futures::enter!("span1");

    async_std::task::sleep(Duration::from_millis(5)).await;
}
