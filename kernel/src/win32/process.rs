//! NT process abstraction for Windows binaries.

use super::objects::Handle;
use alloc::string::String;
use alloc::vec::Vec;

/// Process Environment Block (PEB). Mirrors the Windows `_PEB` layout for the
/// fields a Win32 program inspects early: image base, subsystem, heap list
/// head, and the loader's module list head. Stored inside the process address
/// space at `Process::peb_base`; the kernel keeps a native copy for dispatch.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Peb {
    pub inherited_address_space: u8,
    pub read_image_file_exec_options: u8,
    pub being_debugged: u8,
    pub bit_field: u8,
    pub mutant: u64,
    pub image_base_address: u64,
    pub ldr: u64,
    pub process_parameters: u64,
    pub sub_system_data: u64,
    pub process_heap: u64,
    pub fast_pe_lock: u64,
    pub atl_thunk_s_list_ptr: u64,
    pub ifeo_key: u64,
    pub process_parameters_ptr: u64,
    pub number_of_processors: u32,
    pub nt_global_flag: u32,
}

/// Thread Environment Block (TEB). Mirrors the Windows `_TEB` fields the
/// kernel and user-mode TLS access (gs:[0x30] self pointer on x86_64,
/// TPIDR_EL0 on AArch64) depend on.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Teb {
    pub exception_list: u64,
    pub stack_base: u64,
    pub stack_limit: u64,
    pub sub_system_tib: u64,
    pub fiber_data: u64,
    pub arbitrary_user_pointer: u64,
    pub teb_address: u64,
    pub environment_pointer: u64,
    pub process_id: u64,
    pub thread_id: u64,
    pub active_rpc_handle: u64,
    pub thread_local_storage_pointer: u64,
}

/// A single environment-variable entry (`NAME=VALUE`).
#[derive(Clone)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

/// A Windows process under Aperture OS.
pub struct Process {
    pub pid: u64,
    pub peb_base: u64,
    pub teb_base: u64,
    pub root_handle: Handle,
    /// Base address where the PE image is mapped.
    pub image_base: u64,
    /// Size of the mapped PE image.
    pub image_size: u64,
    /// Absolute entry point address inside the mapped image.
    pub entry_point: u64,
    /// Physical address of the top-level page table (CR3) for this process,
    /// or 0 if the architecture does not use per-process page tables yet.
    pub page_table_root: u64,
    /// Native copy of the PEB the kernel maintains for dispatch.
    pub peb: Peb,
    /// Native copy of the primary thread's TEB.
    pub teb: Teb,
    /// The process environment block (variable-length list of `NAME=VALUE`).
    pub environment: Vec<EnvVar>,
}

impl Process {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            peb_base: 0,
            teb_base: 0,
            root_handle: Handle(0),
            image_base: 0,
            image_size: 0,
            entry_point: 0,
            page_table_root: 0,
            peb: Peb::default(),
            teb: Teb::default(),
            environment: Vec::new(),
        }
    }

    /// Look up an environment variable by name (case-insensitive, matching
    /// Windows `GetEnvironmentVariableW` semantics).
    pub fn get_env(&self, name: &str) -> Option<&str> {
        self.environment
            .iter()
            .find(|v| v.name.eq_ignore_ascii_case(name))
            .map(|v| v.value.as_str())
    }

    /// Set an environment variable, replacing an existing case-insensitive
    /// match, or appending a new entry. Returns the previous value, if any.
    pub fn set_env(&mut self, name: &str, value: &str) -> Option<String> {
        if let Some(slot) = self
            .environment
            .iter_mut()
            .find(|v| v.name.eq_ignore_ascii_case(name))
        {
            let old = core::mem::replace(&mut slot.value, String::from(value));
            return Some(old);
        }
        self.environment.push(EnvVar {
            name: String::from(name),
            value: String::from(value),
        });
        None
    }
}
