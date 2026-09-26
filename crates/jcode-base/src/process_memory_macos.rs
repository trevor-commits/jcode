use super::{OsProcessMemoryInfo, ProcessMemorySnapshot, allocator_info, record_snapshot};
use crate::logging;

pub fn snapshot_with_source(source: impl Into<String>) -> ProcessMemorySnapshot {
    let source = source.into();
    let Some((rss_bytes, peak_rss_bytes, virtual_bytes)) = read_task_sizes() else {
        logging::warn(&format!(
            "process memory snapshot source={source} failed to read Mach task info; using defaults"
        ));
        let snapshot = ProcessMemorySnapshot::default();
        record_snapshot(source, snapshot.clone());
        return snapshot;
    };

    let os = read_physical_footprint().map(
        |(physical_footprint_bytes, peak_physical_footprint_bytes)| OsProcessMemoryInfo {
            pss_bytes: Some(physical_footprint_bytes),
            physical_footprint_bytes: Some(physical_footprint_bytes),
            peak_physical_footprint_bytes: Some(peak_physical_footprint_bytes),
            ..OsProcessMemoryInfo::default()
        },
    );
    if os.is_none() {
        logging::warn(&format!(
            "process memory snapshot source={source} failed to read macOS physical footprint"
        ));
    }

    let snapshot = ProcessMemorySnapshot {
        rss_bytes: Some(rss_bytes),
        peak_rss_bytes: Some(peak_rss_bytes),
        virtual_bytes: Some(virtual_bytes),
        thread_count: None,
        main_stack_bytes: None,
        os,
        allocator: allocator_info(),
    };
    logging::debug(&format!(
        "process memory snapshot source={source} rss={:?} peak_rss={:?} virtual={:?} physical_footprint={:?} allocator={}",
        snapshot.rss_bytes,
        snapshot.peak_rss_bytes,
        snapshot.virtual_bytes,
        snapshot
            .os
            .as_ref()
            .and_then(|info| info.physical_footprint_bytes),
        snapshot.allocator.name
    ));
    record_snapshot(source, snapshot.clone());
    snapshot
}

#[allow(deprecated)]
fn read_task_sizes() -> Option<(u64, u64, u64)> {
    let mut info = std::mem::MaybeUninit::<libc::mach_task_basic_info_data_t>::zeroed();
    let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
    // Safety: `info` is sized for MACH_TASK_BASIC_INFO and `count` describes
    // that buffer in the natural-word units required by task_info.
    let result = unsafe {
        libc::task_info(
            libc::mach_task_self(),
            libc::MACH_TASK_BASIC_INFO as libc::task_flavor_t,
            info.as_mut_ptr().cast::<libc::integer_t>(),
            &mut count,
        )
    };
    if result != libc::KERN_SUCCESS || count < libc::MACH_TASK_BASIC_INFO_COUNT {
        return None;
    }

    // Safety: task_info initialized the full structure and the packed fields
    // are copied with unaligned reads rather than referenced.
    let info = unsafe { info.assume_init() };
    let resident = unsafe { std::ptr::addr_of!(info.resident_size).read_unaligned() };
    let peak_resident = unsafe { std::ptr::addr_of!(info.resident_size_max).read_unaligned() };
    let virtual_size = unsafe { std::ptr::addr_of!(info.virtual_size).read_unaligned() };
    Some((resident, peak_resident, virtual_size))
}

fn read_physical_footprint() -> Option<(u64, u64)> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage_info_v4>::zeroed();
    // Safety: proc_pid_rusage writes a rusage_info_v4 into the provided buffer
    // when called with RUSAGE_INFO_V4.
    let result = unsafe {
        libc::proc_pid_rusage(
            libc::getpid(),
            libc::RUSAGE_INFO_V4,
            usage.as_mut_ptr().cast::<libc::rusage_info_t>(),
        )
    };
    if result != 0 {
        return None;
    }
    // Safety: a zero return means the complete v4 structure was initialized.
    let usage = unsafe { usage.assume_init() };
    Some((
        usage.ri_phys_footprint,
        usage.ri_lifetime_max_phys_footprint,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reports_process_sizes() {
        let snapshot = snapshot_with_source("macos_snapshot_test");

        assert!(snapshot.rss_bytes.is_some_and(|bytes| bytes > 0));
        assert!(snapshot.peak_rss_bytes.is_some_and(|bytes| bytes > 0));
        assert!(snapshot.virtual_bytes.is_some_and(|bytes| bytes > 0));
        let os = snapshot.os.expect("macOS process memory details");
        assert!(os.physical_footprint_bytes.is_some_and(|bytes| bytes > 0));
        assert!(
            os.peak_physical_footprint_bytes
                .is_some_and(|bytes| bytes > 0)
        );
        assert_eq!(os.pss_bytes, os.physical_footprint_bytes);
    }
}
