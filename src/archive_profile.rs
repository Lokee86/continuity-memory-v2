use crate::Archive;
use std::mem::size_of;
use std::time::Duration;

const PROFILE_ENV: &str = "CONTINUITY_PROFILE_ARCHIVE_OPEN";

pub(crate) fn profile_enabled() -> bool {
    std::env::var_os(PROFILE_ENV).is_some()
}

pub(crate) fn report_rebuild(archive: &Archive, scan: Duration, replay: Duration) {
    eprintln!(
        "archive-profile rebuild scan_us={} replay_us={}",
        scan.as_micros(),
        replay.as_micros()
    );
    report_indexes(archive);
}

pub(crate) fn report_open(
    archive: &Archive,
    total: Duration,
    container: Duration,
    rebuild: Duration,
    validate: Duration,
) {
    eprintln!(
        "archive-profile open total_us={} container_us={} rebuild_us={} validate_us={}",
        total.as_micros(),
        container.as_micros(),
        rebuild.as_micros(),
        validate.as_micros()
    );
    eprintln!(
        "archive-profile versions len={} cap={} bytes={}",
        archive.record_versions.len(),
        archive.record_versions.capacity(),
        archive.record_versions.capacity() * size_of::<crate::ArchiveRecordVersion>()
    );
}

fn report_indexes(archive: &Archive) {
    let content_heap = archive.contents.retained_heap_bytes();
    let node_heap = archive.nodes.retained_heap_bytes();
    let branch_heap = archive.branches.retained_heap_bytes();
    let fragment_heap = archive.fragments.retained_heap_bytes();
    let known_heap = content_heap + node_heap + branch_heap + fragment_heap;

    eprintln!(
        "archive-profile indexes contents={}/{}+{} nodes={}/{}+{} branches={}/{}+{} fragments={}/{}+{}",
        archive.contents.len(),
        archive.contents.record_capacity(),
        archive.contents.lookup_capacity(),
        archive.nodes.len(),
        archive.nodes.record_capacity(),
        archive.nodes.lookup_capacity(),
        archive.branches.len(),
        archive.branches.record_capacity(),
        archive.branches.lookup_capacity(),
        archive.fragments.len(),
        archive.fragments.record_capacity(),
        archive.fragments.lookup_capacity()
    );
    eprintln!(
        "archive-profile strings node_values={} branch_values={} fragment_values={}",
        archive.nodes.string_heap_bytes(),
        archive.branches.string_heap_bytes(),
        archive.fragments.string_heap_bytes()
    );
    eprintln!(
        "archive-profile heap_estimates contents={} nodes={} branches={} fragments={} known_heap={}",
        content_heap, node_heap, branch_heap, fragment_heap, known_heap
    );
}
