use crate::{GraphRelation, GraphRelationChange, GraphRelationKind, MemoryId};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct DuplicateTemporalKey {
    pub timestamp_ns: i64,
    pub memory_id: [u8; 32],
}

impl DuplicateTemporalKey {
    pub(crate) fn new(timestamp_ns: i64, memory_id: MemoryId) -> Self {
        Self {
            timestamp_ns,
            memory_id: memory_id.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct DuplicateComponent {
    ordered: BTreeMap<DuplicateTemporalKey, MemoryId>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DuplicateIndex {
    graph_version: Option<u64>,
    components: Vec<Option<DuplicateComponent>>,
    member_component: HashMap<MemoryId, usize>,
}

impl DuplicateIndex {
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    pub(crate) fn graph_version(&self) -> Option<u64> {
        self.graph_version
    }

    pub(crate) fn set_graph_version(&mut self, graph_version: u64) {
        self.graph_version = Some(graph_version);
    }

    pub(crate) fn rebuild(
        graph_version: u64,
        relations: &[GraphRelation],
        keys: &HashMap<MemoryId, DuplicateTemporalKey>,
    ) -> Self {
        let mut adjacency: HashMap<MemoryId, Vec<MemoryId>> = HashMap::new();
        for relation in relations
            .iter()
            .filter(|relation| relation.kind == GraphRelationKind::DuplicateOf && relation.active)
        {
            adjacency
                .entry(relation.source)
                .or_default()
                .push(relation.target);
            adjacency
                .entry(relation.target)
                .or_default()
                .push(relation.source);
        }

        let mut index = Self {
            graph_version: Some(graph_version),
            ..Self::default()
        };
        let mut visited = HashSet::new();
        for start in adjacency.keys().copied() {
            if !visited.insert(start) {
                continue;
            }
            let component_index = index.components.len();
            let mut component = DuplicateComponent::default();
            let mut stack = vec![start];
            while let Some(memory_id) = stack.pop() {
                if let Some(key) = keys.get(&memory_id).copied() {
                    component.ordered.insert(key, memory_id);
                    index.member_component.insert(memory_id, component_index);
                }
                if let Some(neighbors) = adjacency.get(&memory_id) {
                    for neighbor in neighbors.iter().copied() {
                        if visited.insert(neighbor) {
                            stack.push(neighbor);
                        }
                    }
                }
            }
            index.components.push(Some(component));
        }
        index
    }

    pub(crate) fn plan_union(
        &mut self,
        a: MemoryId,
        a_key: DuplicateTemporalKey,
        b: MemoryId,
        b_key: DuplicateTemporalKey,
        relations: &[GraphRelation],
    ) -> Vec<GraphRelationChange> {
        let a_component = self.ensure_member(a, a_key);
        let b_component = self.ensure_member(b, b_key);
        let component = if a_component == b_component {
            a_component
        } else {
            self.merge_components(a_component, b_component)
        };
        self.reconcile_component(component, relations)
    }

    fn ensure_member(&mut self, memory_id: MemoryId, key: DuplicateTemporalKey) -> usize {
        if let Some(component) = self.member_component.get(&memory_id).copied() {
            return component;
        }
        let component = self.components.len();
        let mut value = DuplicateComponent::default();
        value.ordered.insert(key, memory_id);
        self.components.push(Some(value));
        self.member_component.insert(memory_id, component);
        component
    }

    fn merge_components(&mut self, left: usize, right: usize) -> usize {
        let (keep, remove) = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        let removed = self.components[remove]
            .take()
            .expect("duplicate component exists");
        let kept = self.components[keep]
            .as_mut()
            .expect("duplicate component exists");
        for (key, memory_id) in removed.ordered {
            kept.ordered.insert(key, memory_id);
            self.member_component.insert(memory_id, keep);
        }
        keep
    }

    fn reconcile_component(
        &self,
        component: usize,
        relations: &[GraphRelation],
    ) -> Vec<GraphRelationChange> {
        let ordered = &self.components[component]
            .as_ref()
            .expect("duplicate component exists")
            .ordered;
        let members: HashSet<_> = ordered.values().copied().collect();
        let desired: HashSet<_> = ordered
            .values()
            .copied()
            .collect::<Vec<_>>()
            .windows(2)
            .map(|pair| (pair[1], pair[0]))
            .collect();
        let existing: HashSet<_> = relations
            .iter()
            .filter(|relation| {
                relation.active
                    && relation.kind == GraphRelationKind::DuplicateOf
                    && members.contains(&relation.source)
                    && members.contains(&relation.target)
            })
            .map(|relation| (relation.source, relation.target))
            .collect();

        let mut changes = Vec::new();
        for &(source, target) in existing.difference(&desired) {
            changes.push(change(source, target, false));
        }
        for &(source, target) in desired.difference(&existing) {
            changes.push(change(source, target, true));
        }
        changes.sort_by_key(|change| (change.source.0, change.target.0, change.active));
        changes
    }
}

fn change(source: MemoryId, target: MemoryId, active: bool) -> GraphRelationChange {
    GraphRelationChange {
        source,
        target,
        kind: GraphRelationKind::DuplicateOf,
        active,
    }
}
