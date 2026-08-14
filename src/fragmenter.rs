use crate::fragment_store::fragment_id;
use crate::{Archive, ArchiveError, Fragment, FragmentConfig};

impl Archive {
    pub fn materialize_branch_fragments(
        &mut self,
        conversation_id: &str,
        branch_id: &str,
        config: FragmentConfig,
        close_tail: bool,
    ) -> Result<Vec<Fragment>, ArchiveError> {
        let branch = self
            .branches
            .get(conversation_id, branch_id)
            .ok_or(ArchiveError::MissingBranch)?
            .clone();
        self.materialize_path_fragments(conversation_id, &branch.leaf_node_id, config, close_tail)
    }

    pub fn materialize_path_fragments(
        &mut self,
        conversation_id: &str,
        leaf_node_id: &str,
        config: FragmentConfig,
        close_tail: bool,
    ) -> Result<Vec<Fragment>, ArchiveError> {
        validate_config(config)?;
        let nodes = self.branch_nodes(conversation_id, leaf_node_id)?;
        let mut created = Vec::new();
        if nodes.len() >= config.turns {
            let stride = config.turns - config.overlap;
            for start in (0..=nodes.len() - config.turns).step_by(stride) {
                let end = start + config.turns - 1;
                self.materialize_window(&nodes, start, end, &mut created)?;
            }
        }
        if close_tail && !nodes.is_empty() {
            let start = nodes.len().saturating_sub(config.turns);
            self.materialize_window(&nodes, start, nodes.len() - 1, &mut created)?;
        }
        Ok(created)
    }

    fn materialize_window(
        &mut self,
        nodes: &[crate::Node],
        start: usize,
        end: usize,
        created: &mut Vec<Fragment>,
    ) -> Result<(), ArchiveError> {
        let first = &nodes[start];
        let last = &nodes[end];
        let fragment = Fragment {
            id: fragment_id(&first.conversation_id, &first.id, &last.id),
            conversation_id: first.conversation_id.clone(),
            start_node_id: first.id.clone(),
            end_node_id: last.id.clone(),
        };
        if self.put_fragment(fragment.clone())? {
            created.push(fragment);
        }
        Ok(())
    }
}

fn validate_config(config: FragmentConfig) -> Result<(), ArchiveError> {
    if config.turns == 0 || config.overlap >= config.turns {
        return Err(ArchiveError::InvalidFragmentConfig);
    }
    Ok(())
}
