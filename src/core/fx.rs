//! Work off the UI thread. A backend call runs on tokio; its answer comes back as a closure over
//! `&mut App` that `poll` runs on the UI thread, then the view re-runs. This replaces `invoke`.

use std::future::Future;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, OnceLock};

use rune::Waker;

use crate::app::App;

pub type Job = Box<dyn FnOnce(&mut App) + Send>;

/// Cheap to clone; handlers, background tasks and the backend's event sink all hold one.
#[derive(Clone)]
pub struct Fx {
    /// The backend's handle: pass it (or `host.state()`) to `crate::commands::*`.
    pub host: crate::host::Host,
    rt: tokio::runtime::Handle,
    tx: Sender<Job>,
    waker: Arc<OnceLock<Waker>>,
}

impl Fx {
    pub(crate) fn new(host: crate::host::Host, rt: tokio::runtime::Handle) -> (Fx, Receiver<Job>) {
        let (tx, rx) = channel();
        (Fx { host, rt, tx, waker: Arc::default() }, rx)
    }

    pub(crate) fn set_waker(&self, waker: Waker) {
        let _ = self.waker.set(waker);
    }

    /// Run `job` on the UI thread before the next frame. Callable from any thread.
    pub fn post(&self, job: impl FnOnce(&mut App) + Send + 'static) {
        if self.tx.send(Box::new(job)).is_ok() {
            if let Some(wake) = self.waker.get() {
                wake();
            }
        }
    }

    /// Run `fut` on tokio, then `done` with its output on the UI thread.
    ///
    /// ```ignore
    /// let host = app.fx.host.clone();
    /// app.fx.spawn(async move { crate::commands::get_news(false).await }, |app, news| app.home.news = news.ok());
    /// ```
    pub fn spawn<T: Send + 'static>(
        &self,
        fut: impl Future<Output = T> + Send + 'static,
        done: impl FnOnce(&mut App, T) + Send + 'static,
    ) {
        let fx = self.clone();
        self.rt.spawn(async move {
            let out = fut.await;
            fx.post(move |app| done(app, out));
        });
    }

    /// Run `done` on the UI thread once `delay` has passed. The timer is created inside the
    /// runtime: `tokio::time::sleep` panics when built on the UI thread.
    pub fn after(&self, delay: std::time::Duration, done: impl FnOnce(&mut App) + Send + 'static) {
        self.spawn(async move { tokio::time::sleep(delay).await }, move |app, ()| done(app));
    }

    /// Run blocking work (file IO, sync commands that touch disk) on tokio's blocking pool.
    pub fn blocking<T: Send + 'static>(
        &self,
        work: impl FnOnce(&crate::host::Host) -> T + Send + 'static,
        done: impl FnOnce(&mut App, T) + Send + 'static,
    ) {
        let fx = self.clone();
        let host = self.host.clone();
        self.rt.spawn_blocking(move || {
            let out = work(&host);
            fx.post(move |app| done(app, out));
        });
    }

    /// Fire and forget: errors become an error toast.
    pub fn run<T: Send + 'static, E: std::fmt::Display + Send + 'static>(
        &self,
        fut: impl Future<Output = Result<T, E>> + Send + 'static,
    ) {
        self.spawn(fut, |app, result| {
            if let Err(e) = result {
                app.toasts.error(e.to_string());
            }
        });
    }

    pub fn runtime(&self) -> &tokio::runtime::Handle {
        &self.rt
    }
}
