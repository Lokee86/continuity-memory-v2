use reliquary_memory::Container;
use std::{env, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if !(2..=3).contains(&args.len()) {
        return Err("usage: container_open_profile <archive.rel> [runs]".into());
    }
    let runs = args
        .get(2)
        .map(|v| v.parse::<usize>())
        .transpose()?
        .unwrap_or(25);
    let mut timings = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = Instant::now();
        let container = Container::open(&args[1])?;
        timings.push(start.elapsed().as_nanos() as u64);
        std::hint::black_box(container.latest_version());
    }
    timings.sort_unstable();
    println!(
        "container-open runs={} median_us={} p90_us={}",
        runs,
        percentile(&timings, 50) / 1_000,
        percentile(&timings, 90) / 1_000,
    );
    Ok(())
}

fn percentile(values: &[u64], percent: usize) -> u64 {
    let index = ((values.len() - 1) * percent) / 100;
    values[index]
}
