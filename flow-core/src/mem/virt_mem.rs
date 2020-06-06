pub mod virt_from_phys;
pub use virt_from_phys::VirtualFromPhysical;

use crate::error::Result;
use crate::types::{Address, Length, Page, Pointer32, Pointer64};

use std::ffi::CString;
use std::mem::MaybeUninit;

use dataview::Pod;

pub trait VirtualMemory {
    fn virt_read_raw_iter<'a, VI: VirtualReadIterator<'a>>(&mut self, iter: VI) -> Result<()>;

    fn virt_write_raw_iter<'a, VI: VirtualWriteIterator<'a>>(&mut self, iter: VI) -> Result<()>;

    fn virt_page_info(&mut self, addr: Address) -> Result<Page>;

    // read helpers
    fn virt_read_raw_into(&mut self, addr: Address, out: &mut [u8]) -> Result<()> {
        self.virt_read_raw_iter(Some((addr, out)).into_iter())
    }

    fn virt_read_into<T: Pod + ?Sized>(&mut self, addr: Address, out: &mut T) -> Result<()>
    where
        Self: Sized,
    {
        self.virt_read_raw_into(addr, out.as_bytes_mut())
    }

    fn virt_read_raw(&mut self, addr: Address, len: Length) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len.as_usize()];
        self.virt_read_raw_into(addr, &mut *buf)?;
        Ok(buf)
    }

    /// # Safety
    ///
    /// this function will overwrite the contents of 'obj' so we can just allocate an unitialized memory section.
    /// this function should only be used with [repr(C)] structs.
    #[allow(clippy::uninit_assumed_init)]
    fn virt_read<T: Pod + Sized>(&mut self, addr: Address) -> Result<T>
    where
        Self: Sized,
    {
        let mut obj: T = unsafe { MaybeUninit::uninit().assume_init() };
        self.virt_read_into(addr, &mut obj)?;
        Ok(obj)
    }

    // write helpers
    fn virt_write_raw(&mut self, addr: Address, data: &[u8]) -> Result<()> {
        self.virt_write_raw_iter(Some((addr, data)).into_iter())
    }

    fn virt_write<T: Pod + ?Sized>(&mut self, addr: Address, data: &T) -> Result<()>
    where
        Self: Sized,
    {
        self.virt_write_raw(addr, data.as_bytes())
    }

    // specific read helpers
    fn virt_read_addr32(&mut self, addr: Address) -> Result<Address>
    where
        Self: Sized,
    {
        Ok(self.virt_read::<u32>(addr)?.into())
    }

    fn virt_read_addr64(&mut self, addr: Address) -> Result<Address>
    where
        Self: Sized,
    {
        Ok(self.virt_read::<u64>(addr)?.into())
    }

    // read pointer wrappers
    fn virt_read_ptr32_into<U: Pod + ?Sized>(
        &mut self,
        ptr: Pointer32<U>,
        out: &mut U,
    ) -> Result<()>
    where
        Self: Sized,
    {
        self.virt_read_into(ptr.address.into(), out)
    }

    fn virt_read_ptr32<U: Pod + Sized>(&mut self, ptr: Pointer32<U>) -> Result<U>
    where
        Self: Sized,
    {
        self.virt_read(ptr.address.into())
    }

    fn virt_read_ptr64_into<U: Pod + ?Sized>(
        &mut self,
        ptr: Pointer64<U>,
        out: &mut U,
    ) -> Result<()>
    where
        Self: Sized,
    {
        self.virt_read_into(ptr.address.into(), out)
    }

    fn virt_read_ptr64<U: Pod + Sized>(&mut self, ptr: Pointer64<U>) -> Result<U>
    where
        Self: Sized,
    {
        self.virt_read(ptr.address.into())
    }

    // TODO: read into slice?
    // TODO: if len is shorter than string truncate it!
    fn virt_read_cstr(&mut self, addr: Address, len: Length) -> Result<String> {
        let mut buf = vec![0; len.as_usize()];
        self.virt_read_raw_into(addr, &mut buf)?;
        if let Some((n, _)) = buf.iter().enumerate().find(|(_, c)| **c == 0_u8) {
            buf.truncate(n);
        }
        let v = CString::new(buf)?;
        Ok(String::from(v.to_string_lossy()))
    }

    // TODO: chain reading should be obsolete and replaced by something more general
    fn virt_read_addr32_chain(
        &mut self,
        base_addr: Address,
        offsets: Vec<Length>,
    ) -> Result<Address>
    where
        Self: Sized,
    {
        offsets
            .iter()
            .try_fold(base_addr, |c, &a| self.virt_read_addr32(c + a))
    }

    fn virt_read_addr64_chain(
        &mut self,
        base_addr: Address,
        offsets: Vec<Length>,
    ) -> Result<Address>
    where
        Self: Sized,
    {
        offsets
            .iter()
            .try_fold(base_addr, |c, &a| self.virt_read_addr64(c + a))
    }
}

// forward impls
impl<'a, T: VirtualMemory> VirtualMemory for &'a mut T {
    fn virt_read_raw_iter<'b, VI: VirtualReadIterator<'b>>(&mut self, iter: VI) -> Result<()> {
        (*self).virt_read_raw_iter(iter)
    }

    fn virt_write_raw_iter<'b, VI: VirtualWriteIterator<'b>>(&mut self, iter: VI) -> Result<()> {
        (*self).virt_write_raw_iter(iter)
    }

    fn virt_page_info(&mut self, addr: Address) -> Result<Page> {
        (*self).virt_page_info(addr)
    }
}

// iterator helpers
pub type VirtualReadData<'a> = (Address, &'a mut [u8]);
pub trait VirtualReadIterator<'a>: Iterator<Item = VirtualReadData<'a>> + 'a {}
impl<'a, T: Iterator<Item = VirtualReadData<'a>> + 'a> VirtualReadIterator<'a> for T {}

pub type VirtualWriteData<'a> = (Address, &'a [u8]);
pub trait VirtualWriteIterator<'a>: Iterator<Item = VirtualWriteData<'a>> + 'a {}
impl<'a, T: Iterator<Item = VirtualWriteData<'a>> + 'a> VirtualWriteIterator<'a> for T {}