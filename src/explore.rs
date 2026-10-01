//! Safe memory access for reading the game's structures: readable-memory checks (with a small
//! cache of readable regions), raw reads and RTTI class names.

use fromsoftware_shared::UnknownPtr;
use windows::Win32::System::Memory::{MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_NOACCESS, VirtualQuery};


/// Readable regions VirtualQuery reported recently: (start, end, when). Checks inside them don't
/// ask Windows again for REGION_TTL. VirtualQuery made Mario mode run at ~20 fps on native
/// Windows (the collision and clutter scans check memory hundreds of times a frame); most
/// checks land in a few big heap regions.
static REGIONS: std::sync::Mutex<Vec<(usize, usize, std::time::Instant)>> = std::sync::Mutex::new(Vec::new());
const REGION_TTL: std::time::Duration = std::time::Duration::from_millis(500);
const MAX_REGIONS: usize = 64;

/// Memory check statistics (perf log): checks, VirtualQuery calls, time in VirtualQuery (ns).
pub static CHECKS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static QUERIES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static QUERY_NS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// True if `len` bytes at `addr` are committed, readable memory.
pub fn readable(addr: usize, len: usize) -> bool {
    use std::sync::atomic::Ordering::Relaxed;
    if addr < 0x10000 || addr % 8 != 0 {
        return false;
    }
    CHECKS.fetch_add(1, Relaxed);
    let now = std::time::Instant::now();
    let mut regions = REGIONS.lock().unwrap_or_else(|e| e.into_inner());
    regions.retain(|r| now.duration_since(r.2) < REGION_TTL);
    if regions.iter().any(|&(start, end, _)| addr >= start && addr.saturating_add(len) <= end) {
        return true;
    }
    let mut info = MEMORY_BASIC_INFORMATION::default();
    let n = unsafe { VirtualQuery(Some(addr as *const _), &mut info, size_of::<MEMORY_BASIC_INFORMATION>()) };
    QUERIES.fetch_add(1, Relaxed);
    QUERY_NS.fetch_add(now.elapsed().as_nanos() as u64, Relaxed);
    if n == 0 || info.State != MEM_COMMIT {
        return false;
    }
    if info.Protect.0 & (PAGE_NOACCESS.0 | PAGE_GUARD.0) != 0 {
        return false;
    }
    let start = info.BaseAddress as usize;
    let end = start + info.RegionSize;
    if regions.len() >= MAX_REGIONS {
        regions.remove(0);
    }
    regions.push((start, end, now));
    addr + len <= end
}

pub fn read_u64(addr: usize) -> Option<u64> {
    readable(addr, 8).then(|| unsafe { *(addr as *const u64) })
}

/// Class name of the object at `addr` (its first qword must be a vtable with RTTI).
pub fn class_of(addr: usize) -> Option<String> {
    let vt = read_u64(addr)? as usize;
    if !readable(vt.wrapping_sub(8), 16) {
        return None;
    }
    unsafe { UnknownPtr::from(addr) }.rtti_classname()
}
