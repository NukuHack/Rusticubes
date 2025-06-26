
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;

#[cfg(target_os = "linux")]
use libc::{malloc_trim, mallopt, M_ARENA_MAX, M_MMAP_THRESHOLD, M_TOP_PAD};

pub struct MemoryManager;

impl MemoryManager {
    /// Fast memory trim
    pub fn light_trim() {
        // Linux-specific optimization
        #[cfg(target_os = "linux")]
        unsafe {
            malloc_trim(0);
            mallopt(M_ARENA_MAX, 1);  // Reduce memory arenas
        }

        // Fast generic fallback
        Self::fast_pressure(1);  // 1MB pressure
    }

    /// Thorough memory cleanup
    pub fn aggressive_trim() {
        // Linux-specific deep trim
        #[cfg(target_os = "linux")]
        unsafe {
            malloc_trim(0);
            mallopt(M_MMAP_THRESHOLD, 131_072);  // 128KB threshold
            mallopt(M_TOP_PAD, 0);  // Disable top padding
            std::fs::write("/proc/sys/vm/drop_caches", "3").ok();
        }

        // Stronger memory pressure
        Self::fast_pressure(32);  // 32MB pressure
    }

    /// Optimized memory pressure technique
    fn fast_pressure(mb: usize) {
        const BLOCK_SIZE: usize = 1_048_576; // 1MB
        let layout = Layout::from_size_align(BLOCK_SIZE, 8).unwrap();
        let mut blocks = Vec::with_capacity(mb);

        // Allocate and touch memory quickly
        unsafe {
            for _ in 0..mb {
                let block = alloc(layout);
                if !block.is_null() {
                    ptr::write(block as *mut u8, 0); // Single byte touch
                    blocks.push(block);
                }
            }
        }

        // Immediate release
        unsafe {
            for block in blocks {
                dealloc(block, layout);
            }
        }
    }
}

// memory_clean.rs
#[cfg(windows)]
pub fn force_memory_cleanup() {
    use windows_sys::Win32::System::Memory::SetProcessWorkingSetSizeEx;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    
    unsafe {
        let process = GetCurrentProcess();
        SetProcessWorkingSetSizeEx(process as HANDLE, !0, !0, 0);
    }
}

#[cfg(target_os = "linux")]
pub fn force_memory_cleanup() {
    // Drop caches and trim
    std::fs::write("/proc/sys/vm/drop_caches", "3").ok();
    unsafe { libc::malloc_trim(0); }
}

pub fn clean_gpu_memory(device: &wgpu::Device) {
    // Single-pass cleanup
    device.poll(wgpu::Maintain::Wait);
}