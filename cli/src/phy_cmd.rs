use crate::args::PhyCommand;
use crate::util::file_len;
use anyhow::Result;
use reliquary_memory::Phylactery;

pub fn run(command: PhyCommand) -> Result<()> {
    match command {
        PhyCommand::Create { path } => {
            let phy = Phylactery::create(&path)?;
            println!(
                "created PHY: {} id={}",
                path.display(),
                phy.owner_id().as_deref().unwrap_or("none")
            );
        }
        PhyCommand::Info { path } => info(&path)?,
        PhyCommand::Verify { path } => {
            let phy = Phylactery::open(&path)?;
            println!(
                "PHY ok: {} id={} memory_version={} graph_version={}",
                path.display(),
                phy.owner_id().as_deref().unwrap_or("none"),
                phy.memory_version(),
                phy.graph_version()
            );
        }
    }
    Ok(())
}

fn info(path: &std::path::Path) -> Result<()> {
    let phy = Phylactery::open(path)?;
    let memories = phy.memory_stats();
    let graph = phy.graph_stats();
    let packed = phy.packed_vector_stats();
    let vectors = phy.memory_vector_stats();
    let profiles = phy.compatibility_profile_stats();

    println!("path: {}", path.display());
    println!("bytes: {}", file_len(path)?);
    println!("kind: phylactery");
    println!("id: {}", phy.owner_id().as_deref().unwrap_or("none"));
    println!("memory_version: {}", phy.memory_version());
    println!("graph_version: {}", phy.graph_version());
    println!(
        "memories: current={} revisions={} bodies={}",
        memories.memories, memories.revisions, memories.bodies
    );
    println!("graph: active_relations={}", graph.active_relations);
    println!(
        "packed_vectors: objects={} rows={} matrix_bytes={}",
        packed.objects, packed.rows, packed.matrix_bytes
    );
    println!("memory_vectors: bindings={}", vectors.bindings);
    println!("compatibility_profiles: {}", profiles.profiles);
    Ok(())
}
