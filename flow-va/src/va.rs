use std::io::{Error, ErrorKind, Result};

use address::{Address, Length};
use mem::{PhysicalRead, PhysicalWrite, VirtualRead, VirtualWrite};
use arch::Architecture;

use crate::VirtualAddressTranslation;

// TODO: find a cleaner way to do this?
pub struct VatImpl<T>(T);

impl<T: PhysicalRead + VirtualAddressTranslation> VatImpl<T> {
	pub fn new(mem: T) -> Self {
		VatImpl{ 0: mem }
	}
}

// TODO: recover from vtop failures if we request to much memory!
impl<T: PhysicalRead + VirtualAddressTranslation> VirtualRead for VatImpl<T> {
	fn virt_read(&mut self, arch: Architecture, dtb: Address, addr: Address, len: Length) -> Result<Vec<u8>> {
		println!("virt_read(): wrapper!");

        let mut result: Vec<u8> = vec![0; len.as_usize()];

        // TODO: check if comparison works?
        let mut base = addr;
        let end = addr + len;
        while base < end {
            let mut aligned_len = (addr + Length::from_kb(4)).as_page_aligned(arch.instruction_set.page_size()) - addr;
            if base + aligned_len > end {
                aligned_len = end - base;
            }

            let pa = self.0.vtop(arch, dtb, base).unwrap_or(Address::null());
            if !pa.is_null() {
                let mem = self.0.phys_read(pa, aligned_len)?;
                let start = (base - addr).as_usize();
                mem.iter().enumerate().for_each(|(i, b)| {
                    result[start + i] = *b;
                });
            } else {
                // skip
            }

            base += aligned_len;
        }

        Ok(result)

/*
        let pa = self.0.vtop(arch, dtb, addr)?;
        println!("virt_read(): pa={:x}", pa);
        if !pa.is_null() {
            self.phys_read(pa, len)
        } else {
            // TODO: add more debug info
            Err(Error::new(ErrorKind::Other, "virt_read(): readunable to resolve physical address"))
        }
    */
	}
}

impl<T: PhysicalRead + PhysicalWrite + VirtualAddressTranslation> VirtualWrite for VatImpl<T> {
    fn virt_write(&mut self, arch: Architecture, dtb: Address, addr: Address, data: &Vec<u8>) -> Result<Length> {
		println!("virt_write(): wrapper!");

        let pa = self.0.vtop(arch, dtb, addr)?;
        println!("virt_write(): pa={:x}", pa);
        if !pa.is_null() {
            self.0.phys_write(pa, data)
        } else {
            // TODO: add more debug info
            Err(Error::new(ErrorKind::Other, "virt_write(): unable to resolve physical address"))
        }
	}
}