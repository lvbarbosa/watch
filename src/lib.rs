use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

struct Indexed<T> {
    index: usize,
    inner: T,
}

impl<T> Future for Indexed<T>
where
    T: Future + Unpin,
{
    type Output = T::Output;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        Pin::new(&mut self.get_mut().inner).poll(cx)
    }
}

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
pub async fn watch<F, T>(factories: F)
where
    F: IntoIterator,
    F::Item: Fn() -> T,

    T: Future + Unpin,
{
    let factories: Vec<F::Item> = factories.into_iter().collect();

    let mut indexed_tasks: Vec<Indexed<T>> = factories
        .iter()
        .map(|f| f())
        .enumerate()
        .map(|(index, inner)| Indexed { index, inner })
        .collect();

    let mut factory_index: HashMap<usize, F::Item> = factories.into_iter().enumerate().collect();

    loop {
        let (_, stopped_index, other_tasks) = futures::future::select_all(indexed_tasks).await;

        let mut from_to: HashMap<usize, usize> = other_tasks
            .iter()
            .map(|Indexed { index, .. }| index)
            .enumerate()
            .map(|(to, from)| (*from, to))
            .collect();
        from_to.insert(stopped_index, other_tasks.len());

        let mut tasks: Vec<T> = other_tasks
            .into_iter()
            .map(|Indexed { inner, .. }| inner)
            .collect();

        let respawned_task = factory_index
            .get(&stopped_index)
            .expect("invalid index state")();
        tasks.push(respawned_task);

        factory_index = factory_index
            .into_iter()
            .map(|(from, factory)| {
                let to = *from_to.get(&from).expect("invalid index state");
                (to, factory)
            })
            .collect();

        indexed_tasks = tasks
            .into_iter()
            // Might be useful to extract [2]
            .enumerate()
            .map(|(index, inner)| Indexed { index, inner })
            .collect();
    }
}
