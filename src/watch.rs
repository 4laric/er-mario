//! Research tool: a CPU write watchpoint (debug register DR0) on one address, in every thread of
//! the game. When something writes there, the exception handler logs the writing instruction and
//! the game-code return addresses on the stack (who called it), then disarms. Used to find the
//! code that switches a character's ragdoll on.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use windows::Win32::Foundation::{CloseHandle, EXCEPTION_SINGLE_STEP};
use windows::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler, CONTEXT, CONTEXT_DEBUG_REGISTERS_AMD64, EXCEPTION_POINTERS, GetThreadContext, SetThreadContext,
};
use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, GetCurrentThreadId, OpenThread, ResumeThread, SuspendThread, THREAD_GET_CONTEXT, THREAD_SET_CONTEXT,
    THREAD_SUSPEND_RESUME,
};

use crate::log;

static ARMED: AtomicBool = AtomicBool::new(false);
static ADDR: AtomicUsize = AtomicUsize::new(0);
static HITS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// the watched value when armed (to notice a change the watch missed)
static BEFORE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[repr(C, align(16))]
struct Ctx(CONTEXT);

const EXE_LO: u64 = 0x1_4000_0000;
const EXE_HI: u64 = 0x1_5000_0000;

unsafe extern "system" fn handler(info: *mut EXCEPTION_POINTERS) -> i32 {
    const CONTINUE_EXECUTION: i32 = -1;
    const CONTINUE_SEARCH: i32 = 0;
    let info = unsafe { &*info };
    let rec = unsafe { &*info.ExceptionRecord };
    if rec.ExceptionCode != EXCEPTION_SINGLE_STEP || !ARMED.load(Ordering::Relaxed) {
        return CONTINUE_SEARCH;
    }
    let ctx = unsafe { &mut *info.ContextRecord };
    // DR6 bit 0: our DR0 watch fired
    if ctx.Dr6 & 1 == 0 {
        return CONTINUE_SEARCH;
    }
    // the writing instruction is just before RIP (data breakpoints trap after the write)
    let mut line = format!(
        "watch: write to {:#x} at rip {:#x} (thread {}), new value {:#x}; stack:",
        ADDR.load(Ordering::Relaxed),
        ctx.Rip,
        unsafe { GetCurrentThreadId() },
        unsafe { *(ADDR.load(Ordering::Relaxed) as *const u32) }
    );
    // return addresses into the game's code on the stack (the call chain, roughly)
    let rsp = ctx.Rsp as usize;
    let mut found = 0;
    for i in 0..256 {
        let v = unsafe { *((rsp + i * 8) as *const u64) };
        if (EXE_LO..EXE_HI).contains(&v) {
            line += &format!(" {v:#x}");
            found += 1;
            if found >= 14 {
                break;
            }
        }
    }
    HITS.lock().unwrap_or_else(|e| e.into_inner()).push(line);
    ctx.Dr6 = 0;
    // one catch is enough: disarm in this thread (the others are disarmed from the game thread)
    ctx.Dr7 &= !3;
    ARMED.store(false, Ordering::Relaxed);
    CONTINUE_EXECUTION
}

fn set_all_threads(addr: usize, on: bool) -> usize {
    let mut n = 0;
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) else { return 0 };
        let me = GetCurrentThreadId();
        let pid = GetCurrentProcessId();
        let mut te = THREADENTRY32 { dwSize: size_of::<THREADENTRY32>() as u32, ..Default::default() };
        let mut ok = Thread32First(snap, &mut te).is_ok();
        while ok {
            if te.th32OwnerProcessID == pid && te.th32ThreadID != me {
                if let Ok(h) = OpenThread(THREAD_GET_CONTEXT | THREAD_SET_CONTEXT | THREAD_SUSPEND_RESUME, false, te.th32ThreadID) {
                    SuspendThread(h);
                    let mut c = Ctx(CONTEXT { ContextFlags: CONTEXT_DEBUG_REGISTERS_AMD64, ..Default::default() });
                    if GetThreadContext(h, &mut c.0).is_ok() {
                        if on {
                            c.0.Dr0 = addr as u64;
                            // DR0 local enable, break on write (01), 4 bytes (11)
                            c.0.Dr7 = (c.0.Dr7 & !0x000F_0003) | 1 | (0b01 << 16) | (0b11 << 18);
                        } else {
                            c.0.Dr7 &= !0x000F_0003;
                        }
                        c.0.Dr6 = 0;
                        if SetThreadContext(h, &c.0).is_ok() {
                            n += 1;
                        }
                    }
                    ResumeThread(h);
                    let _ = CloseHandle(h);
                }
            }
            ok = Thread32Next(snap, &mut te).is_ok();
        }
        let _ = CloseHandle(snap);
    }
    n
}

/// Arms the watch on `addr` (4 bytes, writes). Call from a game thread.
pub fn arm(addr: usize) {
    if !INSTALLED.swap(true, Ordering::Relaxed) {
        unsafe { AddVectoredExceptionHandler(1, Some(handler)) };
    }
    ADDR.store(addr, Ordering::Relaxed);
    BEFORE.store(unsafe { *(addr as *const u32) }, Ordering::Relaxed);
    ARMED.store(true, Ordering::Relaxed);
    let n = set_all_threads(addr, true);
    log(format!("watch: armed on {addr:#x} in {n} threads"));
}

/// A watch caught something (no more arming this session).
static CAUGHT: AtomicBool = AtomicBool::new(false);

pub fn done() -> bool {
    CAUGHT.load(Ordering::Relaxed)
}

pub fn armed() -> bool {
    ARMED.load(Ordering::Relaxed)
}

/// Every frame: log what the watch caught (and disarm the other threads after a catch).
pub fn poll() {
    // the value changed but the watch didn't fire: CPU watchpoints don't work here (Wine)
    if ARMED.load(Ordering::Relaxed) {
        let a = ADDR.load(Ordering::Relaxed);
        if crate::explore::readable(a & !7, 8) {
            let now = unsafe { *(a as *const u32) };
            if now != BEFORE.load(Ordering::Relaxed) {
                log(format!("watch: {a:#x} changed to {now:#x} WITHOUT the watch firing (no hardware watchpoints here?)"));
                ARMED.store(false, Ordering::Relaxed);
                set_all_threads(0, false);
            }
        }
    }
    let hits = std::mem::take(&mut *HITS.lock().unwrap_or_else(|e| e.into_inner()));
    if hits.is_empty() {
        return;
    }
    for h in hits {
        log(h);
    }
    CAUGHT.store(true, Ordering::Relaxed);
    set_all_threads(0, false);
}
