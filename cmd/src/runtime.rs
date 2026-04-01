use std::future::Future;

use eyre::Result;

pub fn current_thread_runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}

pub fn block_on<T>(future: impl Future<Output = T>) -> Result<T> {
    Ok(current_thread_runtime()?.block_on(future))
}
