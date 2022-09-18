use fcntl::FcntlArg::F_GETFD;
use nix::fcntl::{self, open, OFlag};
use nix::libc::{close, off_t};
use nix::sys::mman::{mmap, munmap, MapFlags, ProtFlags};
use nix::sys::stat::Mode;
use nix::unistd::{sysconf, SysconfVar};

use std::ffi::c_void;
use std::io;
use std::os::unix::prelude::RawFd;
use std::path::Path;
use std::ptr::null_mut;

fn get_page_size() -> Result<usize, io::Error> {
    match sysconf(SysconfVar::PAGE_SIZE) {
        Ok(Some(size)) => Ok(size as usize),
        Ok(None) => {
            eprintln!("PAGE_SIZE variable is not supported");
            return Err(io::Error::from(io::ErrorKind::Unsupported));
        }
        Err(_) => {
            eprintln!("Error occured during the sysconf call");
            return Err(io::Error::from(io::ErrorKind::Other));
        }
    }
}

pub struct MemoryMapping {
    fd: RawFd,
    mapping: *mut c_void,
    len: usize,
}

impl MemoryMapping {
    pub fn new(path: &Path, address: usize, flags: ProtFlags) -> MemoryMapping {
        let fd = Self::prepare_fd(&path).expect("Can't prepare fd");
        let page_size = get_page_size().expect("Can't get page size");
        let mapping =
            Self::prepare_mapping(fd, address, page_size, flags).expect("Can't prepare mapping");

        MemoryMapping {
            fd,
            mapping,
            len: page_size,
        }
    }

    fn prepare_fd(path: &Path) -> Result<RawFd, io::Error> {
        match open(path, OFlag::O_SYNC | OFlag::O_RDWR, Mode::S_IRUSR) {
            Ok(fd) => Ok(fd),
            Err(err) => {
                eprintln!("Can't open {}: {}", path.display(), err);
                Err(io::Error::from(err))
            }
        }
    }

    fn prepare_mapping(
        fd: RawFd,
        address: usize,
        size: usize,
        flags: ProtFlags,
    ) -> Result<*mut c_void, io::Error> {
        let mask = size - 1;

        unsafe {
            match mmap(
                null_mut(),
                size,
                flags,
                MapFlags::MAP_SHARED,
                fd,
                (address & !mask) as off_t,
            ) {
                Ok(ptr) => Ok(ptr),
                Err(errno) => Err(io::Error::from(errno)),
            }
        }
    }
}

impl MemoryMapping {
    pub fn read(&self, address: usize) -> u32 {
        let page_size = get_page_size().expect("Can't get page size");
        let mask = page_size - 1;
        let byte_addr = (self.mapping as usize) + address & mask;

        unsafe {
            let ptr = (self.mapping as *mut u32).offset(byte_addr as isize);
            std::ptr::read(ptr)
        }
    }

    pub fn write(&self, address: usize, value: u32) {
        let page_size = get_page_size().expect("Can't get page size");
        let mask = page_size - 1;
        let byte_addr = (self.mapping as usize) + address & mask;

        unsafe {
            let ptr = (self.mapping as *mut u32).offset(byte_addr as isize);
            std::ptr::write(ptr, value)
        }
    }
}

impl MemoryMapping {
    fn close_fd(&self) {
        if let Ok(ret) = fcntl::fcntl(self.fd, F_GETFD) {
            if ret == self.fd {
                unsafe {
                    close(self.fd);
                }
            }
        }
    }

    fn unmap_mapping(&self) -> Result<(), io::Error> {
        if !self.mapping.is_null() {
            unsafe {
                return match munmap(self.mapping, self.len) {
                    Ok(_) => Ok(()),
                    Err(errno) => Err(io::Error::from(errno)),
                };
            }
        }
        Ok(())
    }
}

impl Drop for MemoryMapping {
    fn drop(&mut self) {
        self.unmap_mapping().ok();
        self.close_fd()
    }
}
