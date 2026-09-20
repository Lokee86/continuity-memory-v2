mod parallel;
mod parse;
mod pipeline;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("import-extraction") if args.len() == 4 => {
            pipeline::import_extraction(&args[1], &args[2], &args[3])?
        }
        Some("resume") if args.len() == 5 => {
            pipeline::resume(&args[1], &args[2], &args[3], &args[4])?
        }
        Some("resume-batch") if args.len() == 6 => {
            pipeline::resume_batch(&args[1], &args[2], &args[3], &args[4], args[5].parse()?)?
        }
        Some("resolve-all") if args.len() == 5 => {
            pipeline::resolve_all(&args[1], &args[2], &args[3], &args[4])?
        }
        Some("resolve-batch") if args.len() == 6 => {
            pipeline::resolve_batch(&args[1], &args[2], &args[3], &args[4], args[5].parse()?)?
        }
        Some("resume-parallel-batch") if args.len() == 7 => parallel::resume_parallel_batch(
            &args[1],
            &args[2],
            &args[3],
            &args[4],
            args[5].parse()?,
            args[6].parse()?,
        )?,
        Some("resolve-target") if args.len() == 7 => {
            pipeline::resolve_target(&args[1], &args[2], &args[3], &args[4], &args[5], &args[6])?
        }
        Some("project-subset") if args.len() == 5 => {
            pipeline::project_subset(&args[1], &args[2], &args[3], &args[4])?
        }
        Some("status") if args.len() == 3 => pipeline::status(&args[1], &args[2])?,
        _ => {
            return Err("usage:
  entity_fixture_pipeline import-extraction <results.jsonl> <project.rel> <user.phy>
  entity_fixture_pipeline resume <config> <results.jsonl> <project.rel> <user.phy>
  entity_fixture_pipeline resume-batch <config> <results.jsonl> <project.rel> <user.phy> <limit>
  entity_fixture_pipeline resolve-all <config> <results.jsonl> <project.rel> <user.phy>
  entity_fixture_pipeline resolve-batch <config> <results.jsonl> <project.rel> <user.phy> <limit>
  entity_fixture_pipeline resume-parallel-batch <config> <results.jsonl> <project.rel> <user.phy> <limit> <workers>
  entity_fixture_pipeline resolve-target <config> <results.jsonl> <project.rel> <user.phy> <memory-id-hex> <mention-text>
  entity_fixture_pipeline project-subset <source28.rel> <source28.phy> <target14.rel> <target14.phy>
  entity_fixture_pipeline status <project.rel> <user.phy>"
                .into());
        }
    }
    Ok(())
}
