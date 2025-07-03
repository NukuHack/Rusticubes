
use std::alloc::{alloc, dealloc, Layout};
use std::ptr;

#[cfg(target_os = "linux")]
use libc::{malloc_trim, mallopt, M_ARENA_MAX, M_MMAP_THRESHOLD, M_TOP_PAD};


/// Lightweight memory optimization
pub fn light_trim() {
    // Platform-specific optimizations
    #[cfg(target_os = "linux")]
    unsafe {
        malloc_trim(0);
        mallopt(M_ARENA_MAX, 1);  // Reduce memory arenas
    }

    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Memory::SetProcessWorkingSetSizeEx;
        use windows_sys::Win32::Foundation::HANDLE;
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        
        let process = GetCurrentProcess();
        SetProcessWorkingSetSizeEx(process as HANDLE, !0, !0, 0);
    }

    // Universal memory pressure
    fast_pressure(1);  // 1MB pressure
}

/// Comprehensive memory cleanup (CPU and GPU)
pub fn hard_clean(device: Option<&wgpu::Device>) {
    // Platform-specific deep cleanup
    #[cfg(target_os = "linux")]
    unsafe {
        malloc_trim(0);
        mallopt(M_MMAP_THRESHOLD, 131_072);  // 128KB threshold
        mallopt(M_TOP_PAD, 0);  // Disable top padding
        std::fs::write("/proc/sys/vm/drop_caches", "3").ok();
    }

    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Memory::SetProcessWorkingSetSizeEx;
        use windows_sys::Win32::Foundation::HANDLE;
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        
        let process = GetCurrentProcess();
        SetProcessWorkingSetSizeEx(process as HANDLE, !0, !0, 0);
    }

    // Stronger memory pressure
    fast_pressure(32);  // 32MB pressure

    // Clean GPU memory if device provided
    if let Some(device) = device {
        device.poll(wgpu::Maintain::Wait);
    }
}

/// Universal memory pressure technique
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

