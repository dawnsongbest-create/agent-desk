use windows_sys::Win32::{
    Foundation::{GlobalFree, HGLOBAL},
    System::{
        DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
        Ole::CF_UNICODETEXT,
    },
};

pub(super) fn copy_authorization(
    window: &tauri::WebviewWindow,
    secret: &str,
) -> Result<(), String> {
    const ERROR: &str = "系统剪贴板不可用，请重试。";
    struct Allocation(HGLOBAL);
    impl Drop for Allocation {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: until SetClipboardData succeeds we own this handle.
                unsafe { GlobalFree(self.0) };
            }
        }
    }
    struct Clipboard;
    impl Drop for Clipboard {
        fn drop(&mut self) {
            // SAFETY: constructed after successful OpenClipboard on this thread.
            unsafe { CloseClipboard() };
        }
    }

    let hwnd = window.hwnd().map_err(|_| ERROR)?.0;
    let utf16: Vec<u16> = secret.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: copy only into a non-null locked allocation of the exact size.
    // Win32 takes ownership only on successful SetClipboardData.
    unsafe {
        let mut memory = Allocation(GlobalAlloc(
            GMEM_MOVEABLE,
            std::mem::size_of_val(utf16.as_slice()),
        ));
        if memory.0.is_null() {
            return Err(ERROR.into());
        }
        let pointer = GlobalLock(memory.0).cast::<u16>();
        if pointer.is_null() {
            return Err(ERROR.into());
        }
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), pointer, utf16.len());
        GlobalUnlock(memory.0);
        if OpenClipboard(hwnd) == 0 {
            return Err(ERROR.into());
        }
        let _clipboard = Clipboard;
        if EmptyClipboard() == 0 || SetClipboardData(u32::from(CF_UNICODETEXT), memory.0).is_null()
        {
            return Err(ERROR.into());
        }
        memory.0 = std::ptr::null_mut();
    }
    Ok(())
}
