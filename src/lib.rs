use std::cmp::Ordering;
use std::collections::HashMap;
use std::future::Future;

/// Spawns and watches all the futures returned by the given [`Future`]
/// factories. Any [`Future`] that gets ready is dropped and a new version of it
/// is spawned.
///
/// A factory is a closure that returns a [`Future`]. It is called every time
/// the [`Future`] it yielded returns an [`std::task::Poll::Ready`]. Outputs are
/// ignored.
///
/// There will never be more than one instance of a task running.
///
/// # Return
///
/// Returns a [`Future`] that never returns. Every time it is polled, it
/// internally polls the `[Future]`'s it is watching.
///
/// # Panic
///
/// This function panics if `factories` is empty.
///
/// # Example
///
/// ```rust
/// use std::future::Future;
/// use std::pin::Pin;
/// use watch::watch;
///
/// async fn task(name: usize) {
///     println!("task {} called", name);
/// }
///
/// fn make_task_factory(name: usize) -> impl Fn() -> Pin<Box<dyn Future<Output = ()>>> {
///     move || Box::pin(task(name))
/// }
///
/// #[tokio::main]
/// async fn main() {
///     watch(vec![make_task_factory(0), make_task_factory(1)]).await;
/// }
/// ```
pub async fn watch<F, T>(factories: F)
where
    F: IntoIterator,
    F::Item: Fn() -> T,

    T: Future + Unpin,
{
    let factories: Vec<F::Item> = factories.into_iter().collect();

    let mut all_tasks: Vec<T> = factories.iter().map(|f| f()).collect();
    let mut factory_index: HashMap<usize, F::Item> = factories.into_iter().enumerate().collect();

    loop {
        let (_, stopped_index, mut other_tasks) = futures::future::select_all(all_tasks).await;
        other_tasks.push(factory_index.get(&stopped_index).unwrap()());
        factory_index =
            update_factory_index_keys(factory_index, stopped_index, other_tasks.len() - 1);
        all_tasks = other_tasks;
    }
}

fn update_factory_index_keys<T>(
    factory_index: HashMap<usize, T>,
    stopped_index: usize,
    respawned_index: usize,
) -> HashMap<usize, T> {
    factory_index
        .into_iter()
        .map(|(k, v)| match k.cmp(&stopped_index) {
            Ordering::Greater => (k - 1, v),
            Ordering::Equal => (respawned_index, v),
            _ => (k, v),
        })
        .collect()
}
