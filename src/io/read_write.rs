//! `read` and `write`, optionally positioned, optionally vectored

#[cfg(any(linux_raw, all(libc, target_os = "linux", target_env = "gnu")))]
use bitflags::bitflags;
use io_lifetimes::{AsFd, BorrowedFd};
#[cfg(all(
    libc,
    not(any(target_os = "android", target_os = "linux", target_os = "emscripten"))
))]
use libc::{pread as libc_pread, pwrite as libc_pwrite};
#[cfg(all(
    libc,
    any(target_os = "android", target_os = "linux", target_os = "emscripten")
))]
use libc::{
    pread64 as libc_pread, preadv64 as libc_preadv, pwrite64 as libc_pwrite,
    pwritev64 as libc_pwritev,
};
#[cfg(all(
    libc,
    not(any(
        target_os = "android",
        target_os = "linux",
        target_os = "emscripten",
        target_os = "redox"
    ))
))]
use libc::{preadv as libc_preadv, pwritev as libc_pwritev};
#[cfg(all(libc, target_os = "linux", target_env = "gnu"))]
use libc::{preadv2 as libc_preadv2, pwritev2 as libc_pwritev2};
#[cfg(libc)]
use libc::{read as libc_read, readv as libc_readv, write as libc_write, writev as libc_writev};
#[cfg(all(libc, not(any(target_os = "redox", target_env = "newlib"))))]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{
    cmp::min,
    convert::TryInto,
    io::{self, IoSlice, IoSliceMut},
};
#[cfg(libc)]
use {crate::negone_err, unsafe_io::os::posish::AsRawFd};

#[cfg(all(libc, target_os = "linux", target_env = "gnu"))]
bitflags! {
    /// `RWF_*` constants.
    pub struct ReadWriteFlags: std::os::raw::c_int {
        /// `RWF_DSYNC`
        const DSYNC = libc::RWF_DSYNC;
        /// `RWF_HIPRI`
        const HIPRI = libc::RWF_HIPRI;
        /// `RWF_SYNC`
        const SYNC = libc::RWF_SYNC;
        /// `RWF_NOWAIT`
        const NOWAIT = libc::RWF_NOWAIT;
        /// `RWF_APPEND`
        const APPEND = libc::RWF_APPEND;
    }
}

#[cfg(linux_raw)]
bitflags! {
    /// `RWF_*` constants.
    pub struct ReadWriteFlags: std::os::raw::c_uint {
        /// `RWF_DSYNC`
        const DSYNC = linux_raw_sys::general::RWF_DSYNC;
        /// `RWF_HIPRI`
        const HIPRI = linux_raw_sys::general::RWF_HIPRI;
        /// `RWF_SYNC`
        const SYNC = linux_raw_sys::general::RWF_SYNC;
        /// `RWF_NOWAIT`
        const NOWAIT = linux_raw_sys::general::RWF_NOWAIT;
        /// `RWF_APPEND`
        const APPEND = linux_raw_sys::general::RWF_APPEND;
    }
}

/// `read(fd, buf.as_ptr(), buf.len())`
#[inline]
pub fn read<'f, Fd: AsFd<'f>>(fd: Fd, buf: &mut [u8]) -> io::Result<usize> {
    let fd = fd.as_fd();
    _read(fd, buf)
}

#[cfg(libc)]
fn _read(fd: BorrowedFd<'_>, buf: &mut [u8]) -> io::Result<usize> {
    let nread = unsafe {
        negone_err(libc_read(
            fd.as_raw_fd() as libc::c_int,
            buf.as_mut_ptr() as *mut _,
            buf.len(),
        ))?
    };
    Ok(nread.try_into().unwrap())
}

#[cfg(linux_raw)]
#[inline]
fn _read(fd: BorrowedFd<'_>, buf: &mut [u8]) -> io::Result<usize> {
    crate::linux_raw::read(fd, buf)
}

/// `write(fd, buf.ptr(), buf.len())`
#[inline]
pub fn write<'f, Fd: AsFd<'f>>(fd: Fd, buf: &[u8]) -> io::Result<usize> {
    let fd = fd.as_fd();
    _write(fd, buf)
}

#[cfg(libc)]
fn _write(fd: BorrowedFd<'_>, buf: &[u8]) -> io::Result<usize> {
    let nwritten = unsafe {
        negone_err(libc_write(
            fd.as_raw_fd() as libc::c_int,
            buf.as_ptr() as *mut _,
            buf.len(),
        ))?
    };
    Ok(nwritten.try_into().unwrap())
}

#[cfg(linux_raw)]
#[inline]
fn _write(fd: BorrowedFd<'_>, buf: &[u8]) -> io::Result<usize> {
    crate::linux_raw::write(fd, buf)
}

/// `pread(fd, buf.as_ptr(), bufs.len(), offset)`
#[inline]
pub fn pread<'f, Fd: AsFd<'f>>(fd: Fd, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    let fd = fd.as_fd();
    _pread(fd, buf, offset)
}

#[cfg(libc)]
fn _pread(fd: BorrowedFd<'_>, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    let offset = offset
        .try_into()
        .map_err(|_overflow_err| io::Error::from_raw_os_error(libc::EOVERFLOW))?;
    let nread = unsafe {
        negone_err(libc_pread(
            fd.as_raw_fd() as libc::c_int,
            buf.as_ptr() as *mut _,
            buf.len(),
            offset,
        ))?
    };
    Ok(nread.try_into().unwrap())
}

#[cfg(linux_raw)]
#[inline]
fn _pread(fd: BorrowedFd<'_>, buf: &[u8], offset: u64) -> io::Result<usize> {
    crate::linux_raw::pread(fd, buf, offset)
}

/// `pwrite(fd, bufs.as_ptr(), bufs.len())`
#[inline]
pub fn pwrite<'f, Fd: AsFd<'f>>(fd: Fd, buf: &[u8], offset: u64) -> io::Result<usize> {
    let fd = fd.as_fd();
    _pwrite(fd, buf, offset)
}

#[cfg(libc)]
fn _pwrite(fd: BorrowedFd<'_>, buf: &[u8], offset: u64) -> io::Result<usize> {
    let offset = offset
        .try_into()
        .map_err(|_overflow_err| io::Error::from_raw_os_error(libc::EOVERFLOW))?;
    let nwritten = unsafe {
        negone_err(libc_pwrite(
            fd.as_raw_fd() as libc::c_int,
            buf.as_ptr() as *mut _,
            buf.len(),
            offset,
        ))?
    };
    Ok(nwritten.try_into().unwrap())
}

#[cfg(linux_raw)]
#[inline]
fn _pwrite(fd: BorrowedFd<'_>, buf: &[u8], offset: u64) -> io::Result<usize> {
    crate::linux_raw::pwrite(fd, buf, offset)
}

/// `readv(fd, bufs.as_ptr(), bufs.len())`
#[inline]
pub fn readv<'f, Fd: AsFd<'f>>(fd: Fd, bufs: &[IoSliceMut]) -> io::Result<usize> {
    let fd = fd.as_fd();
    _readv(fd, bufs)
}

#[cfg(libc)]
fn _readv(fd: BorrowedFd<'_>, bufs: &[IoSliceMut]) -> io::Result<usize> {
    let nread = unsafe {
        negone_err(libc_readv(
            fd.as_raw_fd() as libc::c_int,
            bufs.as_ptr().cast::<libc::iovec>(),
            min(bufs.len(), max_iov()).try_into().unwrap(),
        ))?
    };
    Ok(nread.try_into().unwrap())
}

#[cfg(linux_raw)]
#[inline]
fn _readv(fd: BorrowedFd<'_>, bufs: &[IoSliceMut]) -> io::Result<usize> {
    crate::linux_raw::readv(fd, &bufs[..min(bufs.len(), max_iov())])
}

/// `writev(fd, bufs.as_ptr(), bufs.len())`
#[inline]
pub fn writev<'f, Fd: AsFd<'f>>(fd: Fd, bufs: &[IoSlice]) -> io::Result<usize> {
    let fd = fd.as_fd();
    _writev(fd, bufs)
}

#[cfg(libc)]
fn _writev(fd: BorrowedFd<'_>, bufs: &[IoSlice]) -> io::Result<usize> {
    let nwritten = unsafe {
        negone_err(libc_writev(
            fd.as_raw_fd() as libc::c_int,
            bufs.as_ptr().cast::<libc::iovec>(),
            min(bufs.len(), max_iov()).try_into().unwrap(),
        ))?
    };
    Ok(nwritten.try_into().unwrap())
}

#[cfg(linux_raw)]
#[inline]
fn _writev(fd: BorrowedFd<'_>, bufs: &[IoSlice]) -> io::Result<usize> {
    crate::linux_raw::writev(fd, &bufs[..min(bufs.len(), max_iov())])
}

/// `preadv(fd, bufs.as_ptr(), bufs.len(), offset)`
#[inline]
#[cfg(not(target_os = "redox"))]
pub fn preadv<'f, Fd: AsFd<'f>>(fd: Fd, bufs: &[IoSliceMut], offset: u64) -> io::Result<usize> {
    let fd = fd.as_fd();
    _preadv(fd, bufs, offset)
}

#[cfg(all(libc, not(target_os = "redox")))]
fn _preadv(fd: BorrowedFd<'_>, bufs: &[IoSliceMut], offset: u64) -> io::Result<usize> {
    let offset = offset
        .try_into()
        .map_err(|_overflow_err| io::Error::from_raw_os_error(libc::EOVERFLOW))?;
    let nread = unsafe {
        negone_err(libc_preadv(
            fd.as_raw_fd() as libc::c_int,
            bufs.as_ptr().cast::<libc::iovec>(),
            min(bufs.len(), max_iov()).try_into().unwrap(),
            offset,
        ))?
    };
    Ok(nread.try_into().unwrap())
}

#[cfg(linux_raw)]
#[inline]
fn _preadv(fd: BorrowedFd<'_>, bufs: &[IoSliceMut], offset: u64) -> io::Result<usize> {
    let offset = offset.try_into().map_err(|_overflow_err| {
        io::Error::from_raw_os_error(linux_raw_sys::errno::EOVERFLOW as i32)
    })?;
    crate::linux_raw::preadv(fd, &bufs[..min(bufs.len(), max_iov())], offset)
}

/// `pwritev(fd, bufs.as_ptr(), bufs.len(), offset)`
#[cfg(not(target_os = "redox"))]
#[inline]
pub fn pwritev<'f, Fd: AsFd<'f>>(fd: Fd, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
    let fd = fd.as_fd();
    _pwritev(fd, bufs, offset)
}

#[cfg(all(libc, not(target_os = "redox")))]
fn _pwritev(fd: BorrowedFd<'_>, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
    let offset = offset
        .try_into()
        .map_err(|_overflow_err| io::Error::from_raw_os_error(libc::EOVERFLOW))?;
    let nwritten = unsafe {
        negone_err(libc_pwritev(
            fd.as_raw_fd() as libc::c_int,
            bufs.as_ptr().cast::<libc::iovec>(),
            min(bufs.len(), max_iov()).try_into().unwrap(),
            offset,
        ))?
    };
    Ok(nwritten.try_into().unwrap())
}

#[cfg(linux_raw)]
#[inline]
fn _pwritev(fd: BorrowedFd<'_>, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
    let offset = offset.try_into().map_err(|_overflow_err| {
        io::Error::from_raw_os_error(linux_raw_sys::errno::EOVERFLOW as i32)
    })?;
    crate::linux_raw::pwritev(fd, &bufs[..min(bufs.len(), max_iov())], offset)
}

/// `preadv2(fd, bufs.as_ptr(), bufs.len(), offset, flags)`
#[cfg(any(linux_raw, all(libc, target_os = "linux", target_env = "gnu")))]
#[inline]
pub fn preadv2<'f, Fd: AsFd<'f>>(
    fd: Fd,
    bufs: &[IoSliceMut],
    offset: u64,
    flags: ReadWriteFlags,
) -> io::Result<usize> {
    let fd = fd.as_fd();
    _preadv2(fd, bufs, offset, flags)
}

#[cfg(all(libc, target_os = "linux", target_env = "gnu"))]
fn _preadv2(
    fd: BorrowedFd<'_>,
    bufs: &[IoSliceMut],
    offset: u64,
    flags: ReadWriteFlags,
) -> io::Result<usize> {
    let offset = offset
        .try_into()
        .map_err(|_overflow_err| io::Error::from_raw_os_error(libc::EOVERFLOW))?;
    let nread = unsafe {
        negone_err(libc_preadv2(
            fd.as_raw_fd() as libc::c_int,
            bufs.as_ptr().cast::<libc::iovec>(),
            min(bufs.len(), max_iov()).try_into().unwrap(),
            offset,
            flags.bits(),
        ))?
    };
    Ok(nread.try_into().unwrap())
}

#[cfg(linux_raw)]
#[inline]
fn _preadv2(
    fd: BorrowedFd<'_>,
    bufs: &[IoSliceMut],
    offset: u64,
    flags: ReadWriteFlags,
) -> io::Result<usize> {
    let offset = offset.try_into().map_err(|_overflow_err| {
        io::Error::from_raw_os_error(linux_raw_sys::errno::EOVERFLOW as i32)
    })?;
    crate::linux_raw::preadv2(
        fd,
        &bufs[..min(bufs.len(), max_iov())],
        offset,
        flags.bits(),
    )
}

/// `pwritev2(fd, bufs.as_ptr(), bufs.len(), offset, flags)`
#[cfg(any(linux_raw, all(libc, target_os = "linux", target_env = "gnu")))]
#[inline]
pub fn pwritev2<'f, Fd: AsFd<'f>>(
    fd: Fd,
    bufs: &[IoSlice],
    offset: u64,
    flags: ReadWriteFlags,
) -> io::Result<usize> {
    let fd = fd.as_fd();
    _pwritev2(fd, bufs, offset, flags)
}

#[cfg(all(libc, target_os = "linux", target_env = "gnu"))]
fn _pwritev2(
    fd: BorrowedFd<'_>,
    bufs: &[IoSlice],
    offset: u64,
    flags: ReadWriteFlags,
) -> io::Result<usize> {
    let offset = offset
        .try_into()
        .map_err(|_overflow_err| io::Error::from_raw_os_error(libc::EOVERFLOW))?;
    let nwritten = unsafe {
        negone_err(libc_pwritev2(
            fd.as_raw_fd() as libc::c_int,
            bufs.as_ptr().cast::<libc::iovec>(),
            min(bufs.len(), max_iov()).try_into().unwrap(),
            offset,
            flags.bits(),
        ))?
    };
    Ok(nwritten.try_into().unwrap())
}

#[cfg(linux_raw)]
#[inline]
fn _pwritev2(
    fd: BorrowedFd<'_>,
    bufs: &[IoSlice],
    offset: u64,
    flags: ReadWriteFlags,
) -> io::Result<usize> {
    let offset = offset.try_into().map_err(|_overflow_err| {
        io::Error::from_raw_os_error(linux_raw_sys::errno::EOVERFLOW as i32)
    })?;
    crate::linux_raw::pwritev2(
        fd,
        &bufs[..min(bufs.len(), max_iov())],
        offset,
        flags.bits(),
    )
}

// These functions are derived from Rust's library/std/src/sys/unix/fd.rs at
// revision 108e90ca78f052c0c1c49c42a22c85620be19712.

#[cfg(all(libc, not(any(target_os = "redox", target_env = "newlib"))))]
fn max_iov() -> usize {
    static LIM: AtomicUsize = AtomicUsize::new(0);

    let mut lim = LIM.load(Ordering::Relaxed);
    if lim == 0 {
        let ret = unsafe { libc::sysconf(libc::_SC_IOV_MAX) };

        // 16 is the minimum value required by POSIX.
        lim = if ret > 0 { ret as usize } else { 16 };
        LIM.store(lim, Ordering::Relaxed);
    }

    lim
}

#[cfg(all(libc, any(target_os = "redox", target_env = "newlib")))]
fn max_iov() -> usize {
    16 // The minimum value required by POSIX.
}

#[cfg(linux_raw)]
#[inline]
fn max_iov() -> usize {
    linux_raw_sys::general::UIO_MAXIOV as usize
}
