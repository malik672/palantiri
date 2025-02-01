use alloy_primitives::Bytes;
use crossbeam_skiplist::SkipMap;
use parking_lot::Mutex;
use rayon::prelude::*;
use std::alloc::{GlobalAlloc, Layout};
use std::sync::Arc;
use std::path::Path;
use core_affinity::CoreId;
use std::io;

// The fastest allocator in all of Middle-earth
#[global_allocator]
static SHADOWFAX: ShadowfaxAllocator = ShadowfaxAllocator::new();


// Swift as Shadowfax memory arena
struct ShadowfaxArena {
    chunks: Vec<(*mut u8, usize)>,
    current_chunk: usize,
    chunk_size: usize,
}

// Swift as the Lord of all Horses
#[repr(align(4096))]

struct ShadowfaxMemTable {
    arena: Mutex<ShadowfaxArena>,
    data: SkipMap<Vec<u8>, (Bytes, u64)>,
    size: std::sync::atomic::AtomicUsize,
}

#[cfg(target_os = "macos")]
pub struct ShadowfaxSSTable {
    mmap: memmap2::MmapMut,
    index: Arc<parking_lot::RwLock<dashmap::DashMap<Vec<u8>, u64>>>,
}

// Swift as Shadowfax SSTable
#[cfg(target_os = "linux")]
struct ShadowfaxSSTable {
    mmap: memmap2::MmapMut,
    index: Arc<parking_lot::RwLock<dashmap::DashMap<Vec<u8>, u64>>>,
    ring: io_uring::IoUring,
}

struct ShadowfaxAllocator {
    huge_pages: Mutex<bool>,
}

// Swift as Shadowfax LSM tree
pub struct ShadowfaxLSM {
    memtables: Vec<Arc<ShadowfaxMemTable>>,
    active_memtable: std::sync::atomic::AtomicUsize,
    sstables: Arc<parking_lot::RwLock<Vec<Arc<ShadowfaxSSTable>>>>,
    compaction_trigger: tokio::sync::watch::Sender<()>,
    cpu_cores: Vec<usize>,
}

impl ShadowfaxAllocator {
    const fn new() -> Self {
        Self {
            huge_pages: Mutex::new(true),
        }
    }

    unsafe fn alloc_huge_pages(&self, layout: Layout) -> *mut u8 {
        use libc::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE};
        // Linux-specific huge pages flag
        const MAP_HUGETLB: libc::c_int = 0x40000;

        let size = layout.size();
        let ptr = libc::mmap(
            std::ptr::null_mut(),
            size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB,
            -1,
            0,
        );

        if ptr == libc::MAP_FAILED {
            //slight issue in handling allocation failure
            std::alloc::handle_alloc_error(layout);
        }

        ptr as *mut u8
    }
}

unsafe impl GlobalAlloc for ShadowfaxAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() >= 2 * 1024 * 1024 && *self.huge_pages.lock() {
            self.alloc_huge_pages(layout)
        } else {
            libc::malloc(layout.size()) as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() >= 2 * 1024 * 1024 && *self.huge_pages.lock() {
            libc::munmap(ptr as *mut libc::c_void, layout.size());
        } else {
            libc::free(ptr as *mut libc::c_void);
        }
    }
}

// Safety: We guarantee that the raw pointers in chunks are only accessed
// while holding the mutex, and are never aliased across threads
unsafe impl Send for ShadowfaxArena {}
unsafe impl Sync for ShadowfaxArena {}

impl ShadowfaxArena {
    fn new(chunk_size: usize) -> Self {
        Self {
            chunks: Vec::new(),
            current_chunk: 0,
            chunk_size,
        }
    }

    fn alloc(&mut self, size: usize) -> *mut u8 {
        if self.chunks.is_empty() || size > self.chunks[self.current_chunk].1 {
            let layout = Layout::from_size_align(self.chunk_size, 2 * 1024 * 1024).unwrap();
            let ptr = unsafe { SHADOWFAX.alloc_huge_pages(layout) };
            self.chunks.push((ptr, self.chunk_size));
            self.current_chunk = self.chunks.len() - 1;
        }

        let (ptr, remaining) = &mut self.chunks[self.current_chunk];
        let alloc_ptr = unsafe { ptr.add(self.chunk_size - *remaining) };
        *remaining -= size;
        alloc_ptr
    }
}

impl ShadowfaxMemTable {
    fn new() -> Self {
        Self {
            arena: Mutex::new(ShadowfaxArena::new(1024 * 1024 * 1024)),
            data: SkipMap::new(),
            size: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    // Swift as Shadowfax batch insert
    fn batch_put(&self, entries: Vec<(Vec<u8>, Bytes)>) -> bool {
        let total_size: usize = entries.iter().map(|(k, v)| k.len() + v.len()).sum();

        if self.size.load(std::sync::atomic::Ordering::Relaxed) + total_size > 1024 * 1024 * 1024 {
            return false;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        entries.par_iter().for_each(|(key, value)| {
            //slight issue
            self.data.insert(key.clone(), (value.clone(), timestamp));
        });

        self.size
            .fetch_add(total_size, std::sync::atomic::Ordering::Relaxed);
        true
    }
}

impl ShadowfaxSSTable {
    #[cfg(target_os = "macos")]
    fn new(path: &Path, size: usize) -> io::Result<Self> {

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        file.set_len(size as u64)?;

        let mmap = unsafe { memmap2::MmapOptions::new().map_mut(&file)? };

        Ok(Self {
            mmap,
            index: Arc::new(parking_lot::RwLock::new(dashmap::DashMap::new())),
        })
    }

    #[cfg(target_os = "linux")]
    fn new(path: &std::path::Path, size: usize) -> std::io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        file.set_len(size as u64)?;

        #[cfg(target_os = "linux")]
        unsafe {
            let fd = file.as_raw_fd();
            libc::fcntl(fd, libc::F_SETFL, libc::O_DIRECT);
        }

        let mmap = unsafe {
            memmap2::MmapOptions::new()
                .huge_pages(true)
                .map_mut(&file)?
        };

        Ok(Self {
            mmap,
            index: Arc::new(parking_lot::RwLock::new(dashmap::DashMap::new())),
            ring: io_uring::IoUring::new(1024)?,
        })
    }

    // Swift as Shadowfax read(automated)
    #[cfg(target_arch = "x86_64")]
    unsafe fn vectorized_read(&self, keys: &[Vec<u8>]) -> Vec<Option<Bytes>> {
        use std::arch::x86_64::*;

        let mut results = Vec::with_capacity(keys.len());

        for key in keys {
            let key_bytes = _mm256_loadu_si256(key.as_ptr() as *const __m256i);
            let mut found = None;

            for i in (0..self.mmap.len()).step_by(32) {
                let chunk = _mm256_loadu_si256(self.mmap[i..].as_ptr() as *const __m256i);
                let cmp = _mm256_cmpeq_epi8(chunk, key_bytes);
                let mask = _mm256_movemask_epi8(cmp) as u32;

                if mask != 0 {
                    let pos = i + mask.trailing_zeros() as usize;
                    found = Some(Bytes::copy_from_slice(&self.mmap[pos..pos + 32]));
                    break;
                }
            }

            results.push(found);
        }

        results
    }
}

impl ShadowfaxLSM {
    pub fn new(path: impl AsRef<std::path::Path>) -> Self {
        let cpu_cores = (0..num_cpus::get()).collect();

        let memtables = (0..num_cpus::get())
            .map(|_| Arc::new(ShadowfaxMemTable::new()))
            .collect();

        let (trigger, _) = tokio::sync::watch::channel(());

        Self {
            memtables,
            active_memtable: std::sync::atomic::AtomicUsize::new(0),
            sstables: Arc::new(parking_lot::RwLock::new(Vec::new())),
            compaction_trigger: trigger,
            cpu_cores,
        }
    }
    
    pub async fn batch_write(&self, entries: Vec<(Vec<u8>, Bytes)>) -> std::io::Result<()> {
        let mt_idx = self
            .active_memtable
            .load(std::sync::atomic::Ordering::Relaxed);
        let memtable = &self.memtables[mt_idx];

        if !memtable.batch_put(entries.clone()) {
            self.active_memtable
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let new_idx = self
                .active_memtable
                .load(std::sync::atomic::Ordering::Relaxed)
                % self.memtables.len();

            self.compaction_trigger.send(()).unwrap();

            self.memtables[new_idx].batch_put(entries);
        }

        Ok(())
    }

    pub async fn parallel_read(&self, keys: Vec<Vec<u8>>) -> Vec<Option<Bytes>> {

        let results = keys.chunks(1024).enumerate().map(|(i, chunk)| {
            let core_id = CoreId {
                id: self.cpu_cores[i % self.cpu_cores.len()],
            };
            let chunk = chunk.to_vec();

            tokio::task::spawn_blocking(move || {
                core_affinity::set_for_current(core_id);
                vec![None; chunk.len()]
            })
        });

        let mut final_results = Vec::new();
        for result in futures::future::join_all(results).await {
            final_results.extend(result.unwrap());
        }

        final_results
    }
}
