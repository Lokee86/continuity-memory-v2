use crate::{Archive, ArchiveError, Container, ConversationSummary, ResolvedTurn};
use std::collections::{HashMap, HashSet};

impl Archive {
    pub fn conversation_summaries(&self) -> Vec<ConversationSummary> {
        let mut grouped = HashMap::<&str, Vec<_>>::new();
        for node in self.nodes.iter() {
            grouped
                .entry(node.conversation_id.as_str())
                .or_default()
                .push(node);
        }

        let mut summaries = grouped
            .into_iter()
            .map(|(conversation_id, nodes)| summarize(conversation_id, nodes))
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| {
            right
                .latest_timestamp_ns
                .cmp(&left.latest_timestamp_ns)
                .then_with(|| left.conversation_id.cmp(&right.conversation_id))
        });
        summaries
    }

    pub(crate) fn conversation_turns(
        &self,
        container: &mut Container,
        conversation_id: &str,
        leaf_node_id: &str,
    ) -> Result<Vec<ResolvedTurn>, ArchiveError> {
        self.branch_nodes(conversation_id, leaf_node_id)?
            .into_iter()
            .map(|node| {
                Ok(ResolvedTurn {
                    node_id: node.id,
                    role: node.role,
                    timestamp_ns: node.timestamp_ns,
                    content: self.content(container, node.content_id)?,
                })
            })
            .collect()
    }
}

fn summarize(conversation_id: &str, nodes: Vec<&crate::Node>) -> ConversationSummary {
    let parents = nodes
        .iter()
        .filter_map(|node| node.parent_id.as_deref())
        .collect::<HashSet<_>>();
    let mut leaves = nodes
        .iter()
        .filter(|node| !parents.contains(node.id.as_str()))
        .copied()
        .collect::<Vec<_>>();
    leaves.sort_by(|left, right| {
        right
            .timestamp_ns
            .cmp(&left.timestamp_ns)
            .then_with(|| left.id.cmp(&right.id))
    });

    ConversationSummary {
        conversation_id: conversation_id.to_owned(),
        leaf_node_ids: leaves.into_iter().map(|node| node.id.clone()).collect(),
        turn_count: nodes.len(),
        latest_timestamp_ns: nodes
            .iter()
            .map(|node| node.timestamp_ns)
            .max()
            .unwrap_or_default(),
    }
}
