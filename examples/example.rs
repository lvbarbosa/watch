use std::future::Future;
use std::pin::Pin;
use watch::watch;

async fn task(name: usize) {
    println!("task {} called", name);
    tokio::time::sleep(std::time::Duration::from_secs(1)).await
}

fn make_task_factory(name: usize) -> impl Fn() -> Pin<Box<dyn Future<Output = ()>>> {
    move || Box::pin(task(name))
}

#[tokio::main]
async fn main() {
    watch((0..5).into_iter().map(make_task_factory)).await;
}
