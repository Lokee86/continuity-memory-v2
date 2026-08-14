use continuity_memory::Archive;
use std::alloc::{GlobalAlloc, Layout, System};
use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

struct TrackingAllocator;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            add_bytes(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            if new_size >= layout.size() {
                add_bytes(new_size - layout.size());
            } else {
                CURRENT.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        new_ptr
    }
}

fn add_bytes(bytes: usize) {
    let current = CURRENT.fetch_add(bytes, Ordering::Relaxed) + bytes;
    let mut peak = PEAK.load(Ordering::Relaxed);
    while current > peak {
        match PEAK.compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(found) => peak = found,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if !(2..=3).contains(&args.len()) {
        return Err("usage: archive_open_profile <archive.cva> [runs]".into());
    }
    let runs = args
        .get(2)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(25);

    let mut timings = Vec::with_capacity(runs);
    let mut retained = Vec::with_capacity(runs);
    let mut peaks = Vec::with_capacity(runs);

    for _ in 0..runs {
        let before = CURRENT.load(Ordering::Relaxed);
        PEAK.store(before, Ordering::Relaxed);
        let start = Instant::now();
        let archive = Archive::open(&args[1])?;
        let elapsed = start.elapsed();
        let after = CURRENT.load(Ordering::Relaxed);
        let peak = PEAK.load(Ordering::Relaxed);

        timings.push(elapsed.as_nanos() as u64);
        retained.push(after.saturating_sub(before) as u64);
        peaks.push(peak.saturating_sub(before) as u64);

        std::hint::black_box(archive.stats());
        drop(archive);
    }

    timings.sort_unstable();
    retained.sort_unstable();
    peaks.sort_unstable();
    println!(
        "open-profile runs={} median_us={} p90_us={} retained_median_bytes={} retained_max_bytes={} peak_median_bytes={} peak_max_bytes={}",
        runs,
        percentile(&timings, 50) / 1_000,
        percentile(&timings, 90) / 1_000,
        percentile(&retained, 50),
        *retained.last().unwrap_or(&0),
        percentile(&peaks, 50),
        *peaks.last().unwrap_or(&0)
    );
    Ok(())
}

fn percentile(values: &[u64], percent: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() - 1) * percent) / 100;
    values[index]
}
