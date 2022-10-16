use std::io;
use std::path::Path;
#[cfg(test)]
use std::sync::Once;

use nix::sys::mman::ProtFlags;

use memory_mapping::MemoryMapping;

mod memory_mapping;

pub const DEV_MEM: &str = "/dev/mem";

pub fn read(filepath: &str, address: usize) -> io::Result<u32> {
    let path = Path::new(filepath);
    let mapping = MemoryMapping::new(path, address, ProtFlags::PROT_READ);

    Ok(mapping.read(address))
}

pub fn write(filepath: &str, address: usize, value: u32) -> io::Result<()> {
    let path = Path::new(filepath);
    let mapping = MemoryMapping::new(path, address, ProtFlags::PROT_WRITE);

    mapping.write(address, value);
    Ok(())
}

#[cfg(test)]
const TEST_DEV_MEM: &str = "/tmp/test_dev_mem";
#[cfg(test)]
const TEST_DEV_MEM_SIZE: usize = 1000;
#[cfg(test)]
static INIT: Once = Once::new();

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{Read, Write};
    use std::path::Path;

    use super::*;
    use rand::distributions::Uniform;
    use rand::Rng;

    use crate::TEST_DEV_MEM;

    fn write_random_data_to_file(file_path: &str) -> io::Result<()> {
        let mut rng = rand::thread_rng();
        let range = Uniform::from(0..=u32::MAX);
        let random_data: Vec<u32> = (0..TEST_DEV_MEM_SIZE).map(|_| rng.sample(range)).collect();

        let mut file = File::create(file_path)?;

        for val in &random_data {
            let raw_val: [u8; 4] = unsafe { std::mem::transmute(*val) };
            file.write(&raw_val)?;
        }

        Ok(())
    }

    fn read_random_data_from_file(file_path: &str) -> io::Result<Vec<u32>> {
        let mut file = File::open(file_path)?;
        let mut mem: Vec<u32> = Vec::new();

        for i in 0..TEST_DEV_MEM_SIZE {
            let mut buffer = [0u8; std::mem::size_of::<u32>()];
            file.read_exact(&mut buffer)?;

            let value = unsafe { std::mem::transmute(buffer) };
            mem.insert(i as usize, value);
        }

        Ok(mem)
    }

    fn prepare_test_dev_mem() {
        INIT.call_once(|| {
            if !Path::new(TEST_DEV_MEM).exists() {
                write_random_data_to_file(TEST_DEV_MEM).expect("Can't write random data to file");
            }
        })
    }

    fn read_with_default(addr: usize) -> Option<u32> {
        match read(TEST_DEV_MEM, addr) {
            Ok(val) => Some(val),
            Err(_) => None,
        }
    }

    fn write_with_default(addr: usize, value: u32) -> Result<(), io::Error> {
        write(TEST_DEV_MEM, addr, value)?;
        Ok(())
    }

    fn get_random_address() -> usize {
        let mut rng = rand::thread_rng();

        rng.gen_range(0..TEST_DEV_MEM_SIZE)
    }

    #[test]
    fn read_test() {
        prepare_test_dev_mem();

        match read_random_data_from_file(TEST_DEV_MEM) {
            Ok(mem) => {
                let addr = get_random_address();
                if let Some(right) = mem.get(addr) {
                    assert_eq!(read_with_default(addr).unwrap(), *right);
                }
            }
            Err(err) => {
                panic!("Error({err}) during reading random data from test file");
            }
        }
    }

    #[test]
    fn write_test() {
        prepare_test_dev_mem();

        const WRITE_TEST_VALUE: u32 = 0xDEAD_BEEF;
        let addr = get_random_address();

        if let Err(err) = write_with_default(addr, WRITE_TEST_VALUE) {
            panic!("Error duing writing test value: {}", err);
        }

        assert_eq!(read_with_default(addr).unwrap(), WRITE_TEST_VALUE);
    }
}
