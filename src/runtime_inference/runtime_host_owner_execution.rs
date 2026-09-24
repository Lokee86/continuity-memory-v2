use super::{
    Control, ReliquaryRuntimeHostError, ReliquaryRuntimeRoutes, RuntimeMemoryProfiles, dream,
    insomnia, perception, perception_queue, vectors,
};
use crate::{EpisodePolicy, InsomniaWorkerConfig, InteractionRuntime, Phylactery};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::{self, JoinHandle};

pub(super) struct Shared {
    pub(super) runtime: Arc<Mutex<InteractionRuntime>>,
    pub(super) signal: Arc<(Mutex<Control>, Condvar)>,
    pub(super) routes: Arc<RwLock<ReliquaryRuntimeRoutes>>,
    pub(super) phylactery: Arc<Mutex<Option<Phylactery>>>,
    pub(super) phylactery_enabled: Arc<AtomicBool>,
    pub(super) memory_profiles: Arc<Mutex<RuntimeMemoryProfiles>>,
    pub(super) perception_queue: Arc<Mutex<perception_queue::PerceptionQueue>>,
    pub(super) insomnia_enabled: Arc<AtomicBool>,
    pub(super) insomnia_backpressure_until_ns: Arc<AtomicI64>,
    pub(super) config: InsomniaWorkerConfig,
    pub(super) episode_policy: EpisodePolicy,
}

impl Shared {
    pub(super) fn phylactery_active(&self) -> bool {
        self.phylactery_enabled.load(Ordering::SeqCst)
    }
}

pub(super) struct OwnerExecution {
    pub(super) owner_id: Option<String>,
    pub(super) runtime: Option<Arc<Mutex<InteractionRuntime>>>,
    pub(super) signal: Arc<(Mutex<Control>, Condvar)>,
    pub(super) routes: Arc<RwLock<ReliquaryRuntimeRoutes>>,
    pub(super) phylactery: Arc<Mutex<Option<Phylactery>>>,
    pub(super) phylactery_enabled: Arc<AtomicBool>,
    pub(super) memory_profiles: Arc<Mutex<RuntimeMemoryProfiles>>,
    pub(super) perception_queue: Arc<Mutex<perception_queue::PerceptionQueue>>,
    pub(super) insomnia_enabled: Arc<AtomicBool>,
    pub(super) insomnia_backpressure_until_ns: Arc<AtomicI64>,
    pub(super) episode_policy: EpisodePolicy,
    workers: Vec<JoinHandle<Result<(), ReliquaryRuntimeHostError>>>,
}

impl OwnerExecution {
    pub(super) fn start(
        runtime: InteractionRuntime,
        routes: Arc<RwLock<ReliquaryRuntimeRoutes>>,
        phylactery: Arc<Mutex<Option<Phylactery>>>,
        config: InsomniaWorkerConfig,
        episode_policy: EpisodePolicy,
        insomnia_enabled: bool,
        phylactery_enabled: bool,
    ) -> Self {
        let owner_id = runtime.cva().owner_id();
        let runtime = Arc::new(Mutex::new(runtime));
        let signal = Arc::new((
            Mutex::new(Control {
                stop: false,
                epoch: 1,
            }),
            Condvar::new(),
        ));
        let phylactery_enabled = Arc::new(AtomicBool::new(phylactery_enabled));
        let memory_profiles = Arc::new(Mutex::new(RuntimeMemoryProfiles::default()));
        let perception_queue = Arc::new(Mutex::new(perception_queue::PerceptionQueue::default()));
        let insomnia_enabled = Arc::new(AtomicBool::new(insomnia_enabled));
        let insomnia_backpressure_until_ns = Arc::new(AtomicI64::new(0));
        let shared = Arc::new(Shared {
            runtime: Arc::clone(&runtime),
            signal: Arc::clone(&signal),
            routes: Arc::clone(&routes),
            phylactery: Arc::clone(&phylactery),
            phylactery_enabled: Arc::clone(&phylactery_enabled),
            memory_profiles: Arc::clone(&memory_profiles),
            perception_queue: Arc::clone(&perception_queue),
            insomnia_enabled: Arc::clone(&insomnia_enabled),
            insomnia_backpressure_until_ns: Arc::clone(&insomnia_backpressure_until_ns),
            config: config.clone(),
            episode_policy,
        });
        let mut workers = Vec::with_capacity(config.workers.saturating_add(3));
        for index in 0..config.workers {
            let shared = Arc::clone(&shared);
            workers.push(thread::spawn(move || insomnia::worker_loop(shared, index)));
        }
        {
            let shared = Arc::clone(&shared);
            workers.push(thread::spawn(move || vectors::worker_loop(shared)));
        }
        {
            let shared = Arc::clone(&shared);
            workers.push(thread::spawn(move || dream::worker_loop(shared)));
        }
        {
            let shared = Arc::clone(&shared);
            workers.push(thread::spawn(move || perception::worker_loop(shared)));
        }
        Self {
            owner_id,
            runtime: Some(runtime),
            signal,
            routes,
            phylactery,
            phylactery_enabled,
            memory_profiles,
            perception_queue,
            insomnia_enabled,
            insomnia_backpressure_until_ns,
            episode_policy,
            workers,
        }
    }

    pub(super) fn stop_workers(&mut self) -> Result<(), ReliquaryRuntimeHostError> {
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

    pub(super) fn into_cva(mut self) -> Result<crate::Cva, ReliquaryRuntimeHostError> {
        self.stop_workers()?;
        let runtime = self.runtime.take().ok_or_else(|| {
            ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
        })?;
        let runtime = Arc::try_unwrap(runtime)
            .map_err(|_| ReliquaryRuntimeHostError::Operation("runtime still shared".into()))?
            .into_inner()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        Ok(runtime.into_cva())
    }
}
