use super::SingleFileStore;
use lore_storage::StoreError;

impl SingleFileStore {
    /// Copies immutable Lore associations missing from this artifact.
    ///
    /// Mutable pointers are intentionally excluded: a caller resolving two
    /// divergent artifacts must choose which foreign heads to publish locally.
    pub fn import_immutable_from(&self, source: &Self) -> Result<usize, StoreError> {
        let entries = {
            let index = source.index.read().unwrap();
            index
                .associations
                .iter()
                .map(|((partition, address), fragment)| {
                    let payload = *index.payloads.get(&address.hash).ok_or_else(|| {
                        StoreError::internal("immutable association has no source payload")
                    })?;
                    Ok((*partition, *address, *fragment, payload))
                })
                .collect::<Result<Vec<_>, StoreError>>()?
        };

        let mut imported = 0;
        for (partition, address, fragment, location) in entries {
            if self
                .index
                .read()
                .unwrap()
                .associations
                .contains_key(&(partition, address))
            {
                continue;
            }
            let payload = source.read_payload(location)?;
            self.append_association(partition, address, fragment, Some(payload))?;
            imported += 1;
        }
        Ok(imported)
    }
}
