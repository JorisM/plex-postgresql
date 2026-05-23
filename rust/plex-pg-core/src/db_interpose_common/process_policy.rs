use super::*;

#[cfg(target_os = "linux")]
fn linux_trim_process_name(raw: &str) -> &str {
    raw.trim_matches(char::from(0)).trim()
}

#[cfg(target_os = "linux")]
pub(crate) fn linux_process_name_is_primary(raw: &str) -> bool {
    matches!(
        linux_trim_process_name(raw),
        "Plex Media Scanner" | "Plex Media Server" | "Plex Media Serv"
    )
}

#[cfg(target_os = "linux")]
pub(crate) fn linux_process_name_requires_passthrough(raw: &str) -> bool {
    let name = linux_trim_process_name(raw);
    !name.is_empty() && !linux_process_name_is_primary(name)
}

#[cfg(target_os = "linux")]
fn linux_read_process_comm() -> Option<String> {
    let raw = std::fs::read_to_string(format!("/proc/{}/comm", unsafe { libc::getpid() })).ok()?;
    Some(linux_trim_process_name(&raw).to_string())
}

#[cfg(target_os = "linux")]
pub fn linux_apply_process_role_policy(reason: &str, process_name: &str) -> c_int {
    if !linux_process_name_requires_passthrough(process_name) {
        return 0;
    }

    let name_c = std::ffi::CString::new(linux_trim_process_name(process_name)).ok();
    let reason_c = std::ffi::CString::new(reason).ok();
    unsafe {
        if SHIM_PASSTHROUGH_ONLY
            .compare_exchange(0, 1, Ordering::Release, Ordering::Relaxed)
            .is_ok()
        {
            libc::fprintf(
                stderr_ptr(),
                b"[SHIM_INIT] Linux child role '%s' set to passthrough-only via %s (PID %d)\n\0"
                    .as_ptr() as *const c_char,
                name_c
                    .as_ref()
                    .map(|v| v.as_ptr())
                    .unwrap_or(UNKNOWN_STR.as_ptr() as *const c_char),
                reason_c
                    .as_ref()
                    .map(|v| v.as_ptr())
                    .unwrap_or(UNKNOWN_STR.as_ptr() as *const c_char),
                libc::getpid(),
            );
            libc::fflush(stderr_ptr());
        }
    }

    1
}

#[cfg(target_os = "linux")]
pub fn linux_apply_current_process_role_policy(reason: &str) -> c_int {
    linux_read_process_comm()
        .map(|name| linux_apply_process_role_policy(reason, &name))
        .unwrap_or(0)
}

#[cfg(target_os = "linux")]
pub fn linux_handle_fork_child(_reason: &str) {
    unsafe {
        fast_mark_fork_child_passthrough();
        crate::runtime_linux::disable_postfork_signal_overrides_fast();
        crate::pms_net_compat::disable_for_fork_child_fast();
        // PR_SET_PDEATHSIG triggers on the CALLING THREAD's death, not the
        // process's. When PMS forks helper services (Plex Tuner Service,
        // Plex EAE Service) from a transient worker thread, that worker's
        // subsequent exit signals SIGTERM to the freshly-exec'd helper
        // before it can finish initialising — Tuner Service never writes
        // its log file, PMS's Grabber polls forever, and PMS stays in
        // Maintenance.
        //
        // Opt-in via PLEX_PG_FORK_CHILD_PDEATHSIG=1 if a future deployment
        // needs the auto-reap behaviour. Default off restores Plex's own
        // subreaper-based child management (which is already in place via
        // the LSIO container's `subreaper` wrapper).
        if crate::env_utils::env_truthy(b"PLEX_PG_FORK_CHILD_PDEATHSIG\0") {
            let _ = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
        }
    }
}
