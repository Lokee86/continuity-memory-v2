use crate::{GeneralEndpoint, ReliquaryRuntimeRoutes, SimulatedGeneralEndpoint};
use std::sync::Arc;

fn endpoint(model: &str) -> Arc<dyn GeneralEndpoint> {
    Arc::new(SimulatedGeneralEndpoint::new(model, Vec::new()))
}

#[test]
fn runtime_routes_keep_inference_capabilities_separate() {
    let routes = ReliquaryRuntimeRoutes::new(
        Some(endpoint("general")),
        Some(endpoint("insomnia")),
        Some(endpoint("metadata")),
        Some(endpoint("dream")),
        None,
    );

    assert_eq!(routes.insomnia().unwrap().model(), "insomnia");
    assert_eq!(routes.insomnia_metadata().unwrap().model(), "metadata");
    assert_eq!(routes.insomnia_ownership().unwrap().model(), "metadata");
    assert_eq!(routes.dream().unwrap().model(), "dream");
}

#[test]
fn runtime_routes_apply_fallbacks_inside_reliquary() {
    let routes = ReliquaryRuntimeRoutes::new(Some(endpoint("general")), None, None, None, None);

    assert_eq!(routes.insomnia().unwrap().model(), "general");
    assert_eq!(routes.insomnia_ownership().unwrap().model(), "general");
    assert_eq!(routes.dream().unwrap().model(), "general");
    assert!(routes.insomnia_metadata().is_none());
}
