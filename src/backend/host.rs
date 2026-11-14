//! What the backend needs from the app around it: shared stores, the data dir and somewhere to
//! send progress events. Cheap to clone; the UI hands one to every command.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Path(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

type Sink = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;

struct Inner {
    identifier: String,
    stores: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    sink: RwLock<Option<Sink>>,
}

#[derive(Clone)]
pub struct Host(Arc<Inner>);

/// A store borrowed from the host for as long as a command runs.
pub struct State<'r, T: Send + Sync + 'static>(Arc<T>, PhantomData<&'r T>);

impl<T: Send + Sync + 'static> State<'_, T> {
    pub fn inner(&self) -> &T {
        &self.0
    }
}

impl<T: Send + Sync + 'static> Deref for State<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Send + Sync + 'static> Clone for State<'_, T> {
    fn clone(&self) -> Self {
        State(self.0.clone(), PhantomData)
    }
}

impl Host {
    /// `identifier` names the data dir (`com.ladvace.cactuslauncher`, as the Tauri app used).
    pub fn new(identifier: &str) -> Self {
        Host(Arc::new(Inner { identifier: identifier.into(), stores: RwLock::default(), sink: RwLock::default() }))
    }

    pub fn manage<T: Send + Sync + 'static>(&self, value: T) {
        self.0.stores.write().unwrap().insert(TypeId::of::<T>(), Arc::new(value));
    }

    pub fn state<T: Send + Sync + 'static>(&self) -> State<'_, T> {
        self.try_state().unwrap_or_else(|| panic!("store not managed: {}", std::any::type_name::<T>()))
    }

    pub fn try_state<T: Send + Sync + 'static>(&self) -> Option<State<'_, T>> {
        let any = self.0.stores.read().unwrap().get(&TypeId::of::<T>())?.clone();
        Some(State(any.downcast::<T>().ok()?, PhantomData))
    }

    /// The platform data dir plus the identifier, unless `CACTUS_DATA_DIR` points elsewhere
    /// (a fixture copy for headless runs).
    pub fn app_data_dir(&self) -> Result<PathBuf, Error> {
        if let Some(dir) = std::env::var_os("CACTUS_DATA_DIR").filter(|d| !d.is_empty()) {
            return Ok(PathBuf::from(dir));
        }
        dirs::data_dir()
            .map(|d| d.join(&self.0.identifier))
            .ok_or_else(|| Error::Path("no data directory on this platform".into()))
    }

    /// Where every `emit` lands: event name and its payload as JSON.
    pub fn set_event_sink(&self, sink: impl Fn(&str, serde_json::Value) + Send + Sync + 'static) {
        *self.0.sink.write().unwrap() = Some(Arc::new(sink));
    }

    pub fn emit<S: serde::Serialize + Clone>(&self, event: &str, payload: S) -> Result<(), Error> {
        let sink = self.0.sink.read().unwrap().clone();
        if let Some(sink) = sink {
            sink(event, serde_json::to_value(payload)?);
        }
        Ok(())
    }
}
