use super::{CacheEntry, PageCache, PageType};
use crate::address::{Address, Length};
use crate::arch::Architecture;
use std::alloc::{alloc_zeroed, Layout};

use coarsetime::{Duration, Instant};

// the set page_size must be smaller than the target's page_size, otherwise this would trigger UB
#[derive(Clone)]
pub struct TimedCache {
    address: Box<[Address]>,
    time: Box<[Instant]>,
    cache: Box<[u8]>,
    cache_time: Duration,
    page_size: Length,
    page_type_mask: PageType,
}

impl TimedCache {
    pub fn new(
        arch: Architecture,
        size: Length,
        duration: Duration,
        page_type_mask: PageType,
    ) -> Self {
        let page_size = arch.page_size();
        let cache_entries = size.as_usize() / page_size.as_usize();

        let layout =
            Layout::from_size_align(cache_entries * page_size.as_usize(), page_size.as_usize())
                .unwrap();

        let cache = unsafe {
            Box::from_raw(std::slice::from_raw_parts_mut(
                alloc_zeroed(layout),
                layout.size(),
            ))
        };

        Self {
            address: vec![(!0_u64).into(); cache_entries].into_boxed_slice(),
            time: vec![Instant::now(); cache_entries].into_boxed_slice(),
            cache,
            cache_time: duration,
            page_size,
            page_type_mask,
        }
    }

    fn page_index(&self, addr: Address) -> usize {
        (addr.as_page_aligned(self.page_size).as_usize() / self.page_size.as_usize())
            % self.address.len()
    }

    fn page_and_info_from_index(&mut self, idx: usize) -> (&mut [u8], &mut Address, &mut Instant) {
        let start = self.page_size.as_usize() * idx;
        (
            &mut self.cache[start..(start + self.page_size.as_usize())],
            &mut self.address[idx],
            &mut self.time[idx],
        )
    }

    fn page_from_index(&mut self, idx: usize) -> &mut [u8] {
        let start = self.page_size.as_usize() * idx;
        &mut self.cache[start..(start + self.page_size.as_usize())]
    }

    fn try_page_with_time(
        &mut self,
        addr: Address,
        time: Instant,
    ) -> std::result::Result<&mut [u8], (&mut [u8], &mut Address, &mut Instant)> {
        let page_index = self.page_index(addr);
        if self.address[page_index] == addr.as_page_aligned(self.page_size)
            && time.duration_since(self.time[page_index]) <= self.cache_time
        {
            Ok(self.page_from_index(page_index))
        } else {
            Err(self.page_and_info_from_index(page_index))
        }
    }
}

impl PageCache for TimedCache {
    fn page_size(&self) -> Length {
        self.page_size
    }

    fn is_cached_page_type(&self, page_type: PageType) -> bool {
        self.page_type_mask.contains(page_type)
    }

    fn cached_page_mut(&mut self, addr: Address) -> CacheEntry {
        let page_size = self.page_size;
        let aligned_addr = addr.as_page_aligned(page_size);
        match self.try_page_with_time(addr, Instant::now()) {
            Ok(page) => CacheEntry {
                valid: true,
                address: aligned_addr,
                buf: page,
            },
            Err((page, _, _)) => CacheEntry {
                valid: false,
                address: aligned_addr,
                buf: page,
            },
        }
    }

    fn validate_page(&mut self, addr: Address, page_type: PageType) {
        if self.page_type_mask.contains(page_type) {
            let idx = self.page_index(addr);
            let aligned_addr = addr.as_page_aligned(self.page_size);
            let page_info = self.page_and_info_from_index(idx);
            *page_info.1 = aligned_addr;
            *page_info.2 = Instant::now();
        }
    }

    fn invalidate_page(&mut self, addr: Address, page_type: PageType) {
        if self.page_type_mask.contains(page_type) {
            let idx = self.page_index(addr);
            let page_info = self.page_and_info_from_index(idx);
            *page_info.1 = Address::null();
        }
    }
}