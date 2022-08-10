//! A scoped [`tokio`] Runtime that can be used to create [`Scope`]s which can spawn futures which
//! can access stack data. That is, the futures spawned by the [`Scope`] do not require the `'static`
//! lifetime bound. This can be done safely by ensuring that the [`Scope`] doesn't exit until all
//! spawned futures have finished executing. Be aware, that when a [`Scope`] exits it will block
//! until every future spawned by the [`Scope`] completes. Therefore, one should take caution when
//! created scopes within an asynchronous context, such as from within another spawned future.
//!
//! The futures are also not required to be Send or Sync, making this the most flexible runtime
//! for workloads that aren't computationally bound.
//!
//! # Example
//! ```
//! #[tokio::main]
//! async fn main() {
//!     let mut v = String::from("Hello");
//!
//!     tokio_scoped::scope(|scope| {
//!         // Use the scope to spawn the future.
//!         scope.spawn(async {
//!             v.push('!');
//!         });
//!     });
//!
//!     // The scope won't exit until all spawned futures are complete.
//!     assert_eq!(v.as_str(), "Hello!");
//! }
//! ```
//!
//! See also [`crossbeam::scope`]
//!
//! [`tokio`]: https://tokio.rs/
//! [`crossbeam::scope`]: https://docs.rs/crossbeam/0.4.1/crossbeam/fn.scope.html

use std::{
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::Deref,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use tokio::{
    runtime::{Handle, Runtime},
    sync::mpsc,
    sync::oneshot,
    task::LocalSet,
};

pub struct SpawnHandle<O> {
    pub(crate) _phantom: PhantomData<O>,
}

impl<O> Future for SpawnHandle<O> {
    type Output = O;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        todo!()
        // let inner_future = &mut self.f;
        // let future_ref = Pin::as_mut(inner_future);
        // future_ref.poll(cx)
    }
}

/// Use the current runtime or spawn a new runtime
pub fn scope<'a, O, F: Future<Output = O> + 'a>(
    f: impl FnOnce(&'a Scope) -> F + 'a,
) -> SpawnHandle<O> {
    match Handle::try_current() {
        Ok(handle) => {
            let mut scope = Scope::new(handle, Rc::new(LocalSet::new()));
            // f(&mut scope)

            todo!()
        }
        Err(curerr) => {
            let rt = Runtime::new().unwrap();
            let mut scope = Scope::new(rt.handle().clone(), Rc::new(LocalSet::new()));
            // f(&mut scope)

            todo!()
        }
    }
}

// Boot up a single-threaded runtime on its own thread
pub fn spawn() {

    // rt.block_on();
}

pub fn spawn_local() {}

// Block the current thread
pub fn block_on<'a, R>(f: impl FnOnce(&Scope<'a>)) -> R {
    let rt = Runtime::new().unwrap();

    todo!()
}

// /// Creates a [`Scope`] using the current tokio runtime and calls the `scope` method with the
// /// provided future
// ///
// /// # Example
// /// ```
// /// #[tokio::main]
// /// async fn main() {
// ///     let mut v = String::from("Hello");
// ///     tokio_scoped::scope(|scope| {
// ///         // Use the scope to spawn the future.
// ///         scope.spawn(async {
// ///             v.push('!');
// ///         });
// ///     });
// ///     // The scope won't exit until all spawned futures are complete.
// ///     assert_eq!(v.as_str(), "Hello!");
// /// }
// /// ```
// pub fn scope<'a, F, R>(f: F) -> R
// where
//     F: FnOnce(&mut Scope<'a>) -> R,
// {
//     todo!()
// }

// /// Borrows a `Handle` to the tokio `Runtime` to construct a [`ScopeBuilder`] which can be used to
// /// create a scope.
// ///
// /// # Example
// /// ```
// /// let mut v = String::from("Hello");
// /// let rt = tokio::runtime::Runtime::new().unwrap();
// /// tokio_scoped::scoped(rt.handle()).scope(|scope| {
// ///     // Use the scope to spawn the future.
// ///     scope.spawn(async {
// ///         v.push('!');
// ///     });
// /// });
// /// // The scope won't exit until all spawned futures are complete.
// /// assert_eq!(v.as_str(), "Hello!");
// /// ```
// pub fn scoped(set: &Rc<LocalSet>) -> ScopeBuilder<'_> {
//     ScopeBuilder { set }
// }

/// Struct used to build scopes from a borrowed `Handle`. Generally users should use the [`scoped`]
/// function instead of building `ScopeBuilder` instances directly.
///
/// [`scoped`]: /tokio-scoped/fn.scoped.html
#[derive(Debug)]
pub struct ScopeBuilder<'a> {
    set: &'a Rc<LocalSet>,
}

// impl<'a> ScopeBuilder<'a> {
//     pub fn scope<F, R>(&self, f: F) -> R
//     where
//         F: FnOnce(&mut Scope<'a>) -> R,
//     {
//         let mut scope = Scope::new(self.set.clone());
//         f(&mut scope)
//     }
// }

#[derive(Debug)]
pub struct Scope<'a> {
    handle: Handle,
    set: Rc<LocalSet>,
    send: ManuallyDrop<mpsc::UnboundedSender<()>>,
    // When the `Scope` is dropped, we wait on this receiver to close. No messages are sent through
    // the receiver, however, the `Sender` objects get cloned into each spawned future (see
    // `ScopedFuture`). This is how we ensure they all exit eventually.
    recv: Option<mpsc::UnboundedReceiver<()>>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Scope<'a> {
    fn new<'b: 'a>(handle: Handle, set: Rc<LocalSet>) -> Scope<'a> {
        let (s, r) = mpsc::unbounded_channel();
        Scope {
            handle,
            set,
            send: ManuallyDrop::new(s),
            recv: Some(r),
            _marker: PhantomData,
        }
    }
}

struct ScopedFuture {
    f: Pin<Box<dyn Future<Output = ()> + 'static>>,
    _marker: mpsc::UnboundedSender<()>,
}

impl Future for ScopedFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner_future = &mut self.f;
        let future_ref = Pin::as_mut(inner_future);
        future_ref.poll(cx)
    }
}

impl<'a> Scope<'a> {
    fn scoped_future<'s, F>(&'s self, f: F) -> ScopedFuture
    where
        F: Future<Output = ()> + 'a,
        'a: 's,
    {
        let boxed: Pin<Box<dyn Future<Output = ()> + 'a>> = Box::pin(f);
        // This transmute should be safe, as we use the `ScopedFuture` abstraction to prevent the
        // scope from exiting until every spawned `ScopedFuture` object is dropped, signifying that
        // they have completed their execution.
        let boxed: Pin<Box<dyn Future<Output = ()> + 'static>> =
            unsafe { std::mem::transmute(boxed) };

        ScopedFuture {
            f: boxed,
            _marker: self.send.deref().clone(),
        }
    }

    /// Spawn the provided future on the `Handle` to the tokio `Runtime`.
    pub fn spawn<'s, F, O>(&'s self, future: F) -> tokio::task::JoinHandle<O>
    where
        F: Future<Output = O> + 'a,
        'a: 's,
    {
        todo!()
        // let scoped_f = self.scoped_future(future);
        // self.set.spawn_local(scoped_f)
    }

    /// Creates an `inner` scope which can access variables created within the outer scope.
    pub fn scope<'inner, F, R>(&'inner self, f: F) -> R
    where
        F: FnOnce(&mut Scope<'inner>) -> R,
        'a: 'inner,
    {
        todo!()
        // let mut scope = Scope::new(self.set.clone());
        // f(&mut scope)
    }

    // /// Blocks the "current thread" of the runtime until `future` resolves. Other independently
    // /// spawned futures will be moved to different threads and can make progress while
    // /// this future is running.
    // pub fn block_on<'s, R, F>(&'s mut self, future: F) -> R
    // where
    //     F: Future<Output = R> + Send + 'a,
    //     R: Send + Debug + 'a,
    //     'a: 's,
    // {
    //     let (tx, rx) = oneshot::channel();
    //     let future = async move { tx.send(future.await).unwrap() };

    //     let boxed: Pin<Box<dyn Future<Output = ()> + 'a>> = Box::pin(future);
    //     let boxed: Pin<Box<dyn Future<Output = ()> + 'static>> =
    //         unsafe { std::mem::transmute(boxed) };

    //     self.set.spawn_local(boxed);
    //     {
    //         let handle = self.handle().clone();
    //         tokio::task::block_in_place(move || handle.run_until(rx).await)
    //     }
    // }

    /// Get a `Handle` to the underlying `Runtime` instance.
    pub fn handle(&self) -> &LocalSet {
        &self.set
    }
}

impl<'a> Drop for Scope<'a> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.send);
        }

        let mut recv = self.recv.take().unwrap();
        let n = tokio::task::block_in_place(|| recv.blocking_recv());
        assert_eq!(n, None);
    }
}

#[cfg(test)]
mod testing {
    use super::*;

    use std::time::Duration;
    use tokio::runtime::Runtime;

    #[test]
    fn base() {
        use std::cell::Cell;
        let name = Cell::new(10);

        scope(|scope| async {
            scope.spawn(async {
                name.set(20);
            });

            scope.spawn(async {
                name.set(30);
            });
        });
    }

    // #[test]
    // fn basic_test() {
    //     let rt = make_runtime();
    //     let scoped = scoped(rt.handle());
    //     scoped.scope(|scope| {
    //         scope.spawn(async {
    //             let another = tokio::spawn(async {
    //                 println!("Another!");
    //                 tokio::time::sleep(Duration::from_millis(5000)).await;
    //                 println!("Another is done sleeping");
    //             });

    //             println!("Sleeping a spawned future");
    //             // We should be able to spawn more and also verify that they complete...
    //             tokio::time::sleep(Duration::from_millis(2000)).await;
    //             println!("Completing!");
    //             another.await.unwrap();
    //         });
    //     });
    //     println!("Completed");
    // }

    // #[test]
    // fn basic_test_split() {
    //     let rt = make_runtime();
    //     let scoped = scoped(rt.handle());
    //     scoped.scope(|scope| {
    //         scope.spawn(async {
    //             println!("Another!");
    //             tokio::time::sleep(Duration::from_millis(5000)).await;
    //             println!("Another is done sleeping");
    //         });

    //         println!("Sleeping a spawned future");
    //         scope.spawn(async {
    //             // We should be able to spawn more and also verify that they complete...
    //             tokio::time::sleep(Duration::from_millis(2000)).await;
    //             println!("Completing!");
    //         });
    //     });
    //     println!("Completed");
    // }

    // #[test]
    // fn access_stack() {
    //     let rt = make_runtime();
    //     let scoped = scoped(rt.handle());
    //     // Specifically a variable that does _not_ implement Copy.
    //     let uncopy = String::from("Borrowed!");
    //     scoped.scope(|scope| {
    //         scope.spawn(async {
    //             assert_eq!(uncopy.as_str(), "Borrowed!");
    //             println!("Borrowed successfully: {}", uncopy);
    //         });
    //     });
    // }

    // #[test]
    // fn access_mut_stack() {
    //     let rt = make_runtime();
    //     let scoped = scoped(rt.handle());
    //     let mut uncopy = String::from("Borrowed");
    //     let mut uncopy2 = String::from("Borrowed");
    //     scoped.scope(|scope| {
    //         scope.spawn(async {
    //             let f = scoped.scope(|scope2| scope2.block_on(async { 4 }));
    //             assert_eq!(f, 4);
    //             tokio::time::sleep(Duration::from_millis(1000)).await;
    //             uncopy.push('!');
    //         });

    //         scope.spawn(async {
    //             uncopy2.push('f');
    //         });
    //     });

    //     assert_eq!(uncopy.as_str(), "Borrowed!");
    //     assert_eq!(uncopy2.as_str(), "Borrowedf");
    // }

    // #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    // async fn access_mut_stack_scope_fn() {
    //     let mut uncopy = String::from("Borrowed");
    //     let mut uncopy2 = String::from("Borrowed");
    //     scope(|scope| {
    //         scope.spawn(async {
    //             uncopy.push('!');
    //         });

    //         scope.spawn(async {
    //             uncopy2.push('f');
    //         });
    //     });

    //     assert_eq!(uncopy.as_str(), "Borrowed!");
    //     assert_eq!(uncopy2.as_str(), "Borrowedf");
    // }

    // #[test]
    // fn block_on_test() {
    //     let rt = make_runtime();
    //     let scoped = scoped(rt.handle());
    //     let mut uncopy = String::from("Borrowed");
    //     let captured = scoped.scope(|scope| {
    //         let v = scope
    //             .block_on(async {
    //                 uncopy.push('!');
    //                 Ok::<_, ()>(uncopy)
    //             })
    //             .unwrap();
    //         assert_eq!(v.as_str(), "Borrowed!");
    //         v
    //     });
    //     assert_eq!(captured.as_str(), "Borrowed!");
    // }

    // #[test]
    // fn borrow_many_test() {
    //     let mut values = vec![1, 2, 3, 4];
    //     spawn_off(|scope| async {
    //         for v in &mut values {
    //             scope.spawn(async move {
    //                 *v += 1;
    //             });
    //         }
    //     });

    //     assert_eq!(&values, &[2, 3, 4, 5]);
    // }

    // #[test]
    // fn inner_scope_test() {
    //     let mut values = vec![1, 2, 3, 4];
    //     scope(|scope| {
    //         let mut v2s = vec![2, 3, 4, 5];
    //         scope.scope(|scope2| {
    //             scope2.spawn(async {
    //                 v2s.push(100);
    //                 values.push(100);
    //             });
    //         });
    //         // The inner scope must exit before we can get here.
    //         assert_eq!(v2s, &[2, 3, 4, 5, 100]);
    //         assert_eq!(values, &[1, 2, 3, 4, 100]);
    //     });
    // }

    // #[test]
    // fn borrowed_scope_test() {
    //     let rt = tokio::runtime::Runtime::new().unwrap();
    //     let mut values = vec![1, 2, 3, 4];
    //     ScopeBuilder::from_runtime(&rt).scope(|scope| {
    //         scope.spawn(async {
    //             values.push(100);
    //         });
    //     });
    //     assert_eq!(values, &[1, 2, 3, 4, 100]);
    // }
}
