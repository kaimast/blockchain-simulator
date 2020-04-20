use blockchain_simulator::server::{NullCallback, main_thread};
use blockchain_simulator::TestOperation;

use std::sync::Arc;

fn main() {
    let mut rt = tokio::runtime::Builder::new()
        .enable_io()
        .enable_time()
        .threaded_scheduler()
        .build().expect("Failed to start worker threads");

    let callback = Arc::new( NullCallback{} );

    rt.block_on(async move {
        main_thread::<TestOperation>(callback).await;
    });
}
