//! Ajustes de escalonamento da thread de input.
//!
//! No Windows: registra a thread no MMCSS (tarefa "Games", que reduz a latência de
//! escalonamento) e sobe a prioridade para `ABOVE_NORMAL`. Em outros sistemas não faz nada.
//!
//! É o único módulo desta crate com `unsafe`: duas chamadas FFI sem ponteiros retidos.

/// Resultado dos ajustes (para log; falhar não impede o loop de funcionar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RealtimeTuning {
    /// A thread foi registrada no MMCSS.
    pub mmcss: bool,
    /// A prioridade foi elevada.
    pub priority: bool,
}

#[cfg(windows)]
#[allow(unsafe_code)]
pub fn tune_current_thread() -> RealtimeTuning {
    use windows_sys::Win32::System::Threading::{
        AvSetMmThreadCharacteristicsW, GetCurrentThread, SetThreadPriority,
        THREAD_PRIORITY_ABOVE_NORMAL,
    };

    // "Games\0" em UTF-16.
    let task: Vec<u16> = "Games\0".encode_utf16().collect();
    let mut task_index = 0u32;
    // SAFETY: `task` é uma string UTF-16 terminada em zero que vive durante a chamada e
    // `task_index` é um `u32` válido para escrita.
    let mmcss =
        !unsafe { AvSetMmThreadCharacteristicsW(task.as_ptr(), &raw mut task_index) }.is_null();
    // SAFETY: `GetCurrentThread` devolve um pseudo-handle sempre válido para a própria thread.
    let priority =
        unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_ABOVE_NORMAL) } != 0;
    RealtimeTuning { mmcss, priority }
}

#[cfg(not(windows))]
pub fn tune_current_thread() -> RealtimeTuning {
    RealtimeTuning::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuning_a_scratch_thread_does_not_panic() {
        // Numa thread própria, para não mexer na prioridade da thread de teste.
        let tuning = std::thread::spawn(tune_current_thread).join().unwrap();
        #[cfg(windows)]
        assert!(
            tuning.priority,
            "SetThreadPriority deveria funcionar sem privilégios"
        );
        #[cfg(not(windows))]
        assert_eq!(tuning, RealtimeTuning::default());
    }
}
