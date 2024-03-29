use blockchain_simulator::server::{main_thread, NullCallback};
use blockchain_simulator::TestOperation;

use std::sync::Arc;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("Failed to start worker threads");

    let callback = Arc::new(NullCallback {});

    rt.block_on(async move {
        println!("Started blockchain test server");
        main_thread::<TestOperation>(callback).await;
    });
}
