use crate::dream_source_time::reliquary_source_timestamp_ns;
use crate::dream_temporal_parser::parse_temporal;
use crate::{Cva, DreamTemporalAnalysis, Memory, MemoryError, MemoryId};

impl Cva {
    pub fn dream_temporal_analysis(
        &mut self,
        memory_id: MemoryId,
    ) -> Result<DreamTemporalAnalysis, MemoryError> {
        let memory = self.memories.memory(&mut self.container, memory_id)?;
        let source_timestamp_ns = reliquary_source_timestamp_ns(&self.archive, &memory);
        Ok(analyze_memory_temporal(&memory, source_timestamp_ns))
    }
}

pub(crate) fn analyze_memory_temporal(
    memory: &Memory,
    source_timestamp_ns: Option<i64>,
) -> DreamTemporalAnalysis {
    parse_temporal(
        &format!("{}\n{}", memory.title, memory.content),
        source_timestamp_ns,
    )
}
