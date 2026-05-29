//! Peer validation for Unix domain sockets.

use std::os::unix::io::AsRawFd;
use tokio::net::UnixStream;

/// Validate that the peer process is one of the allowed program names.
#[cfg(target_os = "linux")]
pub fn validate(stream: &UnixStream, allowed: &[&str]) -> bool {
    let fd = stream.as_raw_fd();

    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;

    let ret = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut cred as *mut libc::ucred as *mut libc::c_void,
            &mut len,
        )
    };

    if ret != 0 {
        log::warn!("SO_PEERCRED getsockopt failed");
        return false;
    }

    let comm_path = format!("/proc/{}/comm", cred.pid);
    match std::fs::read_to_string(&comm_path) {
        Ok(name) => {
            let name = name.trim();
            let ok = allowed.contains(&name);
            if !ok {
                log::warn!("Peer pid={} comm='{}' not in allowed list", cred.pid, name);
            }
            ok
        }
        Err(e) => {
            log::warn!("Cannot read {}: {}", comm_path, e);
            false
        }
    }
}

/// Validate that the peer belongs to the same local user.
#[cfg(target_os = "macos")]
pub fn validate(stream: &UnixStream, _allowed: &[&str]) -> bool {
    let fd = stream.as_raw_fd();
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    let ret = unsafe { libc::getpeereid(fd, &mut uid, &mut gid) };
    if ret != 0 {
        log::warn!("getpeereid failed");
        return false;
    }

    let current_uid = unsafe { libc::geteuid() };
    let ok = uid == current_uid;
    if !ok {
        log::warn!("Peer uid={} gid={} does not match current uid={}", uid, gid, current_uid);
    }
    ok
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn validate(_stream: &UnixStream, _allowed: &[&str]) -> bool {
    false
}
