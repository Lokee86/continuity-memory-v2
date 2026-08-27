use crate::{
    EmbeddingEndpoint, EpisodePolicy, GeneralEndpoint, InsomniaWorkerConfig, InteractionRuntime,
    Phylactery,
};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[path = "runtime_host_insomnia.rs"]
mod insomnia;
#[path = "runtime_host_interaction.rs"]
mod interaction;
#[path = "runtime_host_vectors.rs"]
mod vectors;

pub(super) const IDLE_POLL: Duration = Duration::from_secs(1);
pub(super) const VECTOR_RETRY_POLL: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub enum ReliquaryRuntimeHostError {
    Operation(String),
    LockPoisoned,
    ThreadPanicked,
}

impl std::fmt::Display for ReliquaryRuntimeHostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operation(message) => f.write_str(message),
            Self::LockPoisoned => f.write_str("Reliquary runtime host lock is poisoned"),
            Self::ThreadPanicked => f.write_str("Reliquary runtime host worker panicked"),
        }
    }
}

impl std::error::Error for ReliquaryRuntimeHostError {}

pub(super) struct Control {
    pub(super) stop: bool,
    pub(super) epoch: u64,
}

pub(super) struct Shared {
    pub(super) runtime: Arc<Mutex<InteractionRuntime>>,
    pub(super) signal: Arc<(Mutex<Control>, Condvar)>,
    pub(super) general_endpoint: Arc<RwLock<Option<Arc<dyn GeneralEndpoint>>>>,
    pub(super) embedding_endpoint: Arc<RwLock<Option<Arc<dyn EmbeddingEndpoint + Send + Sync>>>>,
    pub(super) phylactery: Arc<Mutex<Option<Phylactery>>>,
    pub(super) config: InsomniaWorkerConfig,
    pub(super) episode_policy: EpisodePolicy,
}

pub struct ReliquaryRuntimeHost {
    pub(crate) runtime: Option<Arc<Mutex<InteractionRuntime>>>,
    signal: Arc<(Mutex<Control>, Condvar)>,
    general_endpoint: Arc<RwLock<Option<Arc<dyn GeneralEndpoint>>>>,
    embedding_endpoint: Arc<RwLock<Option<Arc<dyn EmbeddingEndpoint + Send + Sync>>>>,
    phylactery: Arc<Mutex<Option<Phylactery>>>,
    workers: Vec<JoinHandle<Result<(), ReliquaryRuntimeHostError>>>,
}

impl ReliquaryRuntimeHost {
    pub fn start(
        runtime: InteractionRuntime,
        general_endpoint: Option<Arc<dyn GeneralEndpoint>>,
        embedding_endpoint: Option<Arc<dyn EmbeddingEndpoint + Send + Sync>>,
        config: InsomniaWorkerConfig,
        episode_policy: EpisodePolicy,
    ) -> Self {
        Self::start_inner(
            runtime,
            None,
            general_endpoint,
            embedding_endpoint,
            config,
            episode_policy,
        )
    }

    pub fn start_with_phylactery(
        runtime: InteractionRuntime,
        phylactery: Phylactery,
        general_endpoint: Option<Arc<dyn GeneralEndpoint>>,
        embedding_endpoint: Option<Arc<dyn EmbeddingEndpoint + Send + Sync>>,
        config: InsomniaWorkerConfig,
        episode_policy: EpisodePolicy,
    ) -> Self {
        Self::start_inner(
            runtime,
            Some(phylactery),
            general_endpoint,
            embedding_endpoint,
            config,
            episode_policy,
        )
    }

    fn start_inner(
        runtime: InteractionRuntime,
        phylactery: Option<Phylactery>,
        general_endpoint: Option<Arc<dyn GeneralEndpoint>>,
        embedding_endpoint: Option<Arc<dyn EmbeddingEndpoint + Send + Sync>>,
        config: InsomniaWorkerConfig,
        episode_policy: EpisodePolicy,
    ) -> Self {
        let runtime = Arc::new(Mutex::new(runtime));
        let signal = Arc::new((
            Mutex::new(Control {
                stop: false,
                epoch: 1,
            }),
            Condvar::new(),
        ));
        let general_endpoint = Arc::new(RwLock::new(general_endpoint));
        let embedding_endpoint = Arc::new(RwLock::new(embedding_endpoint));
        let phylactery = Arc::new(Mutex::new(phylactery));
        let shared = Arc::new(Shared {
            runtime: Arc::clone(&runtime),
            signal: Arc::clone(&signal),
            general_endpoint: Arc::clone(&general_endpoint),
            embedding_endpoint: Arc::clone(&embedding_endpoint),
            phylactery: Arc::clone(&phylactery),
            config: config.clone(),
            episode_policy,
        });
        let mut workers = Vec::with_capacity(config.workers.saturating_add(1));
        for index in 0..config.workers {
            let shared = Arc::clone(&shared);
            workers.push(thread::spawn(move || insomnia::worker_loop(shared, index)));
        }
        {
            let shared = Arc::clone(&shared);
            workers.push(thread::spawn(move || vectors::worker_loop(shared)));
        }
        Self {
            runtime: Some(runtime),
            signal,
            general_endpoint,
            embedding_endpoint,
            phylactery,
            workers,
        }
    }

    pub fn set_general_endpoint(
        &self,
        endpoint: Option<Arc<dyn GeneralEndpoint>>,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        *self
            .general_endpoint
            .write()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)? = endpoint;
        self.wake()
    }

    pub fn set_embedding_endpoint(
        &self,
        endpoint: Option<Arc<dyn EmbeddingEndpoint + Send + Sync>>,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        *self
            .embedding_endpoint
            .write()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)? = endpoint;
        self.wake()
    }

    pub fn wake(&self) -> Result<(), ReliquaryRuntimeHostError> {
        notify_work(&self.signal)
    }

    pub fn into_cva(self) -> Result<crate::Cva, ReliquaryRuntimeHostError> {
        self.into_cva_and_phylactery().map(|(cva, _)| cva)
    }

    pub fn into_cva_and_phylactery(
        mut self,
    ) -> Result<(crate::Cva, Option<Phylactery>), ReliquaryRuntimeHostError> {
        self.stop_workers()?;
        let phylactery = self
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .take();
        let runtime = self.runtime.take().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
        })?;
        let runtime = Arc::try_unwrap(runtime)
            .map_err(|_| ReliquaryRuntimeHostError::Operation("runtime still shared".into()))?
            .into_inner()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        Ok((runtime.into_cva(), phylactery))
    }

    fn stop_workers(&mut self) -> Result<(), ReliquaryRuntimeHostError> {
        let (lock, signal) = &*self.signal;
        {
            let mut control = lock
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            control.stop = true;
            signal.notify_all();
        }
        for worker in self.workers.drain(..) {
            worker
                .join()
                .map_err(|_| ReliquaryRuntimeHostError::ThreadPanicked)??;
        }
        Ok(())
    }
}

impl Drop for ReliquaryRuntimeHost {
    fn drop(&mut self) {
        let _ = self.stop_workers();
    }
}

pub(super) fn stopped(
    signal: &(Mutex<Control>, Condvar),
) -> Result<bool, ReliquaryRuntimeHostError> {
    signal
        .0
        .lock()
        .map(|state| state.stop)
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)
}

pub(super) fn wait_for_work(
    signal: &(Mutex<Control>, Condvar),
    seen_epoch: u64,
) -> Result<u64, ReliquaryRuntimeHostError> {
    wait_for_work_timeout(signal, seen_epoch, IDLE_POLL)
}

pub(super) fn wait_for_work_timeout(
    signal: &(Mutex<Control>, Condvar),
    seen_epoch: u64,
    timeout: Duration,
) -> Result<u64, ReliquaryRuntimeHostError> {
    let mut control = signal
        .0
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    if control.epoch == seen_epoch && !control.stop {
        let (next, _) = signal
            .1
            .wait_timeout(control, timeout)
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        control = next;
    }
    Ok(control.epoch)
}

pub(super) fn notify_work(
    signal: &(Mutex<Control>, Condvar),
) -> Result<(), ReliquaryRuntimeHostError> {
    let mut control = signal
        .0
        .lock()
        .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
    control.epoch = control.epoch.wrapping_add(1);
    signal.1.notify_all();
    Ok(())
}

pub(super) fn operation(error: impl std::fmt::Display) -> ReliquaryRuntimeHostError {
    ReliquaryRuntimeHostError::Operation(error.to_string())
}
