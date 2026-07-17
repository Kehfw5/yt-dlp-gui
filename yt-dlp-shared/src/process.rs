//! 操作系统级进程控制模块（挂起/恢复/终止）

// ========== Windows 进程控制 ==========

#[cfg(target_os = "windows")]
mod win32 {
    #[repr(C)]
    pub struct THREADENTRY32 {
        pub dw_size: u32,
        pub cnt_usage: u32,
        pub th32_thread_id: u32,
        pub th32_owner_process_id: u32,
        pub tp_base_pri: i32,
        pub tp_delta_pri: i32,
        pub dw_flags: u32,
    }

    #[repr(C)]
    pub struct PROCESSENTRY32W {
        pub dw_size: u32,
        pub cnt_usage: u32,
        pub th32_process_id: u32,
        pub th32_default_heap_id: usize,
        pub th32_module_id: u32,
        pub cnt_threads: u32,
        pub th32_parent_process_id: u32,
        pub pc_pri_class_base: i32,
        pub dw_flags: u32,
        pub sz_exe_file: [u16; 260],
    }

    pub const TH32CS_SNAPTHREAD: u32 = 0x00000004;
    pub const TH32CS_SNAPPROCESS: u32 = 0x00000002;
    pub const THREAD_SUSPEND_RESUME: u32 = 0x0002;

    extern "system" {
        pub fn CreateToolhelp32Snapshot(dw_flags: u32, th32_process_id: u32) -> isize;
        pub fn Thread32First(h_snapshot: isize, lpte: *mut THREADENTRY32) -> i32;
        pub fn Thread32Next(h_snapshot: isize, lpte: *mut THREADENTRY32) -> i32;
        pub fn Process32FirstW(h_snapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        pub fn Process32NextW(h_snapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        pub fn OpenThread(
            dw_desired_access: u32,
            b_inherit_handle: i32,
            dw_thread_id: u32,
        ) -> isize;
        pub fn SuspendThread(h_thread: isize) -> u32;
        pub fn ResumeThread(h_thread: isize) -> u32;
        pub fn CloseHandle(h_object: isize) -> i32;
    }
}

/// 递归收集指定 PID 及其所有子进程的 PID
#[cfg(target_os = "windows")]
fn collect_process_tree(root_pid: u32) -> std::collections::HashSet<u32> {
    let mut pid_set = std::collections::HashSet::new();
    pid_set.insert(root_pid);

    // SAFETY: 调用 Win32 API CreateToolhelp32Snapshot + Process32FirstW/NextW 遍历系统进程表。
    unsafe {
        use win32::*;
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == -1 {
            return pid_set;
        }
        let mut entry = std::mem::zeroed::<PROCESSENTRY32W>();
        entry.dw_size = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        let mut all_procs = Vec::new();
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                all_procs.push((entry.th32_process_id, entry.th32_parent_process_id));
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);

        let mut queue = vec![root_pid];
        let mut i = 0;
        while i < queue.len() {
            let parent = queue[i];
            for &(pid, ppid) in &all_procs {
                if ppid == parent && pid_set.insert(pid) {
                    queue.push(pid);
                }
            }
            i += 1;
        }
    }
    pid_set
}

/// 挂起指定 PID 的进程及其所有子进程
#[cfg(target_os = "windows")]
pub fn suspend_process(pid: u32) -> Result<(), String> {
    let pids = collect_process_tree(pid);

    unsafe {
        use win32::*;
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == -1 {
            return Err("err_create_thread_snapshot".into());
        }
        let mut entry = std::mem::zeroed::<THREADENTRY32>();
        entry.dw_size = std::mem::size_of::<THREADENTRY32>() as u32;
        if Thread32First(snapshot, &mut entry) != 0 {
            loop {
                if pids.contains(&entry.th32_owner_process_id) {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32_thread_id);
                    if thread != 0 {
                        SuspendThread(thread);
                        CloseHandle(thread);
                    }
                }
                if Thread32Next(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn suspend_process(pid: u32) -> Result<(), String> {
    std::process::Command::new("kill")
        .args(["-STOP", &pid.to_string()])
        .output()
        .map_err(|e| format!("err_suspend_process:{}", e))?;
    Ok(())
}

/// 恢复指定 PID 的进程及其所有子进程
#[cfg(target_os = "windows")]
pub fn resume_process(pid: u32) -> Result<(), String> {
    let pids = collect_process_tree(pid);

    unsafe {
        use win32::*;
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == -1 {
            return Err("err_create_thread_snapshot".into());
        }
        let mut entry = std::mem::zeroed::<THREADENTRY32>();
        entry.dw_size = std::mem::size_of::<THREADENTRY32>() as u32;
        if Thread32First(snapshot, &mut entry) != 0 {
            loop {
                if pids.contains(&entry.th32_owner_process_id) {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32_thread_id);
                    if thread != 0 {
                        ResumeThread(thread);
                        CloseHandle(thread);
                    }
                }
                if Thread32Next(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn resume_process(pid: u32) -> Result<(), String> {
    std::process::Command::new("kill")
        .args(["-CONT", &pid.to_string()])
        .output()
        .map_err(|e| format!("err_resume_process:{}", e))?;
    Ok(())
}

/// 终止指定 PID 的进程及其子进程
#[cfg(target_os = "windows")]
pub fn kill_process(pid: u32) -> Result<(), String> {
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    use std::os::windows::process::CommandExt;
    std::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("err_kill_process:{}", e))?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn kill_process(pid: u32) -> Result<(), String> {
    std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output()
        .map_err(|e| format!("err_kill_process:{}", e))?;
    Ok(())
}
