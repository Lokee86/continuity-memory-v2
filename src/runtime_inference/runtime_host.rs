use crate::{
    CompatibilityProfileId, DecisionEndpoint, EmbeddingEndpoint, EpisodePolicy, GeneralEndpoint,
    InsomniaWorkerConfig, InteractionRuntime, Phylactery,
};
use std::sync::atomic::{AtomicBool, AtomicI64};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[path = "runtime_host_archive_search.rs"]
mod archive_search;
#[path = "runtime_host_dream.rs"]
mod dream;
#[path = "runtime_host_dream_owner.rs"]
mod dream_owner;
#[path = "runtime_host_files.rs"]
mod files;
#[path = "runtime_host_insomnia.rs"]
mod insomnia;
#[path = "runtime_host_interaction.rs"]
mod interaction;
#[path = "runtime_host_knowledge.rs"]
mod knowledge;
#[path = "runtime_host_knowledge_memory.rs"]
mod knowledge_memory;
#[path = "runtime_host_knowledge_relation.rs"]
mod knowledge_relation;
#[path = "runtime_host_memory_provenance.rs"]
mod memory_provenance;
#[path = "runtime_host_memory_search.rs"]
pub mod memory_search;
#[path = "runtime_host_perception.rs"]
mod perception;
#[path = "runtime_host_perception_api.rs"]
mod perception_api;
#[path = "runtime_host_perception_owner.rs"]
mod perception_owner;
#[path = "runtime_host_perception_queue.rs"]
mod perception_queue;
#[path = "runtime_host_status.rs"]
pub mod status;
#[path = "runtime_host_vectors.rs"]
mod vectors;

pub(super) const IDLE_POLL: Duration = Duration::from_secs(1);
pub(super) const DREAM_RETRY_POLL: Duration = Duration::from_secs(60);
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

#[derive(Clone, Copy, Default)]
pub(super) struct RuntimeMemoryProfiles {
    pub(super) project: Option<CompatibilityProfileId>,
    pub(super) user: Option<CompatibilityProfileId>,
}

#[derive(Clone, Default)]
pub struct ReliquaryRuntimeRoutes {
    general: Option<Arc<dyn GeneralEndpoint>>,
    insomnia: Option<Arc<dyn GeneralEndpoint>>,
    insomnia_metadata: Option<Arc<dyn GeneralEndpoint>>,
    entity_extraction: Option<Arc<dyn GeneralEndpoint>>,
    entity_resolution_decision: Option<Arc<dyn DecisionEndpoint>>,
    entity_resolution: Option<Arc<dyn GeneralEndpoint>>,
    chronos: Option<Arc<dyn GeneralEndpoint>>,
    dream: Option<Arc<dyn GeneralEndpoint>>,
    embedding: Option<Arc<dyn EmbeddingEndpoint + Send + Sync>>,
}

impl ReliquaryRuntimeRoutes {
    pub fn new(
        general: Option<Arc<dyn GeneralEndpoint>>,
        insomnia: Option<Arc<dyn GeneralEndpoint>>,
        insomnia_metadata: Option<Arc<dyn GeneralEndpoint>>,
        dream: Option<Arc<dyn GeneralEndpoint>>,
        embedding: Option<Arc<dyn EmbeddingEndpoint + Send + Sync>>,
    ) -> Self {
        Self {
            general,
            insomnia,
            insomnia_metadata,
            entity_extraction: None,
            entity_resolution_decision: None,
            entity_resolution: None,
            chronos: None,
            dream,
            embedding,
        }
    }

    pub(crate) fn insomnia(&self) -> Option<Arc<dyn GeneralEndpoint>> {
        self.insomnia.clone().or_else(|| self.general.clone())
    }

    pub(crate) fn insomnia_metadata(&self) -> Option<Arc<dyn GeneralEndpoint>> {
        self.insomnia_metadata.clone()
    }

    pub(crate) fn insomnia_ownership(&self) -> Option<Arc<dyn GeneralEndpoint>> {
        self.insomnia_metadata().or_else(|| self.insomnia())
    }

    pub fn with_entity_routes(
        mut self,
        extraction: Option<Arc<dyn GeneralEndpoint>>,
        resolution: Option<Arc<dyn GeneralEndpoint>>,
    ) -> Self {
        self.entity_extraction = extraction;
        self.entity_resolution = resolution;
        self
    }

    pub(crate) fn entity_extraction(&self) -> Option<Arc<dyn GeneralEndpoint>> {
        self.entity_extraction.clone()
    }

    pub fn with_entity_resolution_decision(
        mut self,
        endpoint: Option<Arc<dyn DecisionEndpoint>>,
    ) -> Self {
        self.entity_resolution_decision = endpoint;
        self
    }

    pub(crate) fn entity_resolution_decision(&self) -> Option<Arc<dyn DecisionEndpoint>> {
        self.entity_resolution_decision.clone()
    }

    pub(crate) fn entity_resolution(&self) -> Option<Arc<dyn GeneralEndpoint>> {
        self.entity_resolution.clone()
    }

    pub fn with_chronos(mut self, endpoint: Option<Arc<dyn GeneralEndpoint>>) -> Self {
        self.chronos = endpoint;
        self
    }

    pub(crate) fn chronos(&self) -> Option<Arc<dyn GeneralEndpoint>> {
        self.chronos.clone().or_else(|| self.insomnia())
    }

    pub(crate) fn dream(&self) -> Option<Arc<dyn GeneralEndpoint>> {
        self.dream.clone().or_else(|| self.general.clone())
    }

    pub fn embedding(&self) -> Option<Arc<dyn EmbeddingEndpoint + Send + Sync>> {
        self.embedding.clone()
    }
}

pub(super) struct Shared {
    pub(super) runtime: Arc<Mutex<InteractionRuntime>>,
    pub(super) signal: Arc<(Mutex<Control>, Condvar)>,
    pub(super) routes: Arc<RwLock<ReliquaryRuntimeRoutes>>,
    pub(super) phylactery: Arc<Mutex<Option<Phylactery>>>,
    pub(super) memory_profiles: Arc<Mutex<RuntimeMemoryProfiles>>,
    pub(super) perception_queue: Arc<Mutex<perception_queue::PerceptionQueue>>,
    pub(super) insomnia_enabled: Arc<AtomicBool>,
    pub(super) insomnia_backpressure_until_ns: Arc<AtomicI64>,
    pub(super) config: InsomniaWorkerConfig,
    pub(super) episode_policy: EpisodePolicy,
}

pub struct ReliquaryRuntimeHost {
    pub(crate) runtime: Option<Arc<Mutex<InteractionRuntime>>>,
    signal: Arc<(Mutex<Control>, Condvar)>,
    routes: Arc<RwLock<ReliquaryRuntimeRoutes>>,
    phylactery: Arc<Mutex<Option<Phylactery>>>,
    memory_profiles: Arc<Mutex<RuntimeMemoryProfiles>>,
    perception_queue: Arc<Mutex<perception_queue::PerceptionQueue>>,
    insomnia_enabled: Arc<AtomicBool>,
    insomnia_backpressure_until_ns: Arc<AtomicI64>,
    episode_policy: EpisodePolicy,
    workers: Vec<JoinHandle<Result<(), ReliquaryRuntimeHostError>>>,
}

impl ReliquaryRuntimeHost {
    pub fn start(
        runtime: InteractionRuntime,
        routes: ReliquaryRuntimeRoutes,
        config: InsomniaWorkerConfig,
        episode_policy: EpisodePolicy,
    ) -> Self {
        Self::start_inner(runtime, None, routes, config, episode_policy, true)
    }

    pub fn start_inactive(
        runtime: InteractionRuntime,
        routes: ReliquaryRuntimeRoutes,
        config: InsomniaWorkerConfig,
        episode_policy: EpisodePolicy,
    ) -> Self {
        Self::start_inner(runtime, None, routes, config, episode_policy, false)
    }

    pub fn start_with_phylactery(
        runtime: InteractionRuntime,
        phylactery: Phylactery,
        routes: ReliquaryRuntimeRoutes,
        config: InsomniaWorkerConfig,
        episode_policy: EpisodePolicy,
    ) -> Self {
        Self::start_inner(
            runtime,
            Some(phylactery),
            routes,
            config,
            episode_policy,
            true,
        )
    }

    fn start_inner(
        runtime: InteractionRuntime,
        phylactery: Option<Phylactery>,
        routes: ReliquaryRuntimeRoutes,
        config: InsomniaWorkerConfig,
        episode_policy: EpisodePolicy,
        insomnia_enabled: bool,
    ) -> Self {
        let runtime = Arc::new(Mutex::new(runtime));
        let signal = Arc::new((
            Mutex::new(Control {
                stop: false,
                epoch: 1,
            }),
            Condvar::new(),
        ));
        let routes = Arc::new(RwLock::new(routes));
        let phylactery = Arc::new(Mutex::new(phylactery));
        let memory_profiles = Arc::new(Mutex::new(RuntimeMemoryProfiles::default()));
        let perception_queue = Arc::new(Mutex::new(perception_queue::PerceptionQueue::default()));
        let insomnia_enabled = Arc::new(AtomicBool::new(insomnia_enabled));
        let insomnia_backpressure_until_ns = Arc::new(AtomicI64::new(0));
        let shared = Arc::new(Shared {
            runtime: Arc::clone(&runtime),
            signal: Arc::clone(&signal),
            routes: Arc::clone(&routes),
            phylactery: Arc::clone(&phylactery),
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
            runtime: Some(runtime),
            signal,
            routes,
            phylactery,
            memory_profiles,
            perception_queue,
            insomnia_enabled,
            insomnia_backpressure_until_ns,
            episode_policy,
            workers,
        }
    }

    pub fn set_routes(
        &self,
        routes: ReliquaryRuntimeRoutes,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        *self
            .routes
            .write()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)? = routes;
        self.insomnia_backpressure_until_ns
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.wake()
    }

    pub fn wake(&self) -> Result<(), ReliquaryRuntimeHostError> {
        notify_work(&self.signal)
    }

    pub fn set_insomnia_enabled(&self, enabled: bool) -> Result<(), ReliquaryRuntimeHostError> {
        self.insomnia_enabled
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
        self.wake()
    }

    pub fn rel_metadata(&self) -> Result<crate::RelMetadata, ReliquaryRuntimeHostError> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        Ok(runtime.cva.rel_metadata())
    }

    pub fn set_rel_metadata(
        &self,
        type_label: Option<String>,
        dependencies: Vec<String>,
    ) -> Result<bool, ReliquaryRuntimeHostError> {
        let mut runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| {
                ReliquaryRuntimeHostError::Operation("Reliquary runtime is unavailable".into())
            })?
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        let changed = runtime
            .cva
            .set_rel_metadata(type_label, dependencies)
            .map_err(operation)?;
        if changed {
            runtime.cva.sync().map_err(operation)?;
        }
        Ok(changed)
    }

    pub fn has_phylactery(&self) -> Result<bool, ReliquaryRuntimeHostError> {
        self.phylactery
            .lock()
            .map(|slot| slot.is_some())
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)
    }

    pub fn attach_phylactery(
        &self,
        mut phylactery: Phylactery,
    ) -> Result<(), ReliquaryRuntimeHostError> {
        let mut slot = self
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
        if slot.is_some() {
            return Err(ReliquaryRuntimeHostError::Operation(
                "Reliquary runtime already has a Phylactery attached".into(),
            ));
        }
        let perception_keys = perception_owner::startup_keys_user(&mut phylactery)?;
        *slot = Some(phylactery);
        drop(slot);
        {
            let mut queue = self
                .perception_queue
                .lock()
                .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?;
            queue.clear(perception_queue::PerceptionOwner::User);
            queue.push_all(perception_queue::PerceptionOwner::User, perception_keys);
        }
        self.wake()
    }

    pub fn detach_phylactery(&self) -> Result<Option<Phylactery>, ReliquaryRuntimeHostError> {
        let phylactery = self
            .phylactery
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .take();
        self.memory_profiles
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .user = None;
        self.perception_queue
            .lock()
            .map_err(|_| ReliquaryRuntimeHostError::LockPoisoned)?
            .clear(perception_queue::PerceptionOwner::User);
        self.wake()?;
        Ok(phylactery)
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
