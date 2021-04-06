use std::{path::Path, time::Duration};

fn main() {
    wgpu_subscriber::initialize_default_subscriber(Some(Path::new("chrome.json")));

    profiling_futures::enter_unguarded!("spawning");
    let handle = async_std::task::spawn(some_work());
    profiling_futures::exit!();

    profiling_futures::enter_unguarded!("waiting");
    async_std::task::block_on(handle);
    profiling_futures::exit!();
}

#[profiling_futures::wrap]
async fn some_work() {
    profiling_futures::enter!("span");
    profiling_futures::enter!("span1");

    async_std::task::sleep(Duration::from_millis(5)).await;
}
