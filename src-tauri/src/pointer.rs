//! While Mundwerk listens, the mouse pointer carries a red microphone (`tools/gen_pointer.py` draws it), so a running
//! dictation shows where one looks. Windows only: the arrow and the text cursor of the whole system are swapped for
//! it, and the system's own come back when listening ends, when Mundwerk quits and when it starts (after a crash the
//! swapped ones would stay until then).

#[cfg(windows)]
pub use windows_pointer::{listening, reset, restore};

/// Other systems keep their pointer.
#[cfg(not(windows))]
pub fn listening() {}

#[cfg(not(windows))]
pub fn restore() {}

#[cfg(not(windows))]
pub fn reset() {}

#[cfg(windows)]
mod windows_pointer {
    use std::sync::atomic::{AtomicBool, Ordering};

    use windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject, HGDIOBJ};
    use windows::Win32::UI::WindowsAndMessaging::{
        CopyIcon, CreateIconIndirect, DestroyIcon, SetSystemCursor, SystemParametersInfoW, HCURSOR, ICONINFO,
        OCR_IBEAM, OCR_NORMAL, SPI_SETCURSORS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    };

    /// 32 x 32 pixels, BGRA, top row first; the hot spot is the arrow's tip at the top left.
    const PIXELS: &[u8] = include_bytes!("../icons/pointer-listening.bgra");
    const SIZE: i32 = 32;

    // Whether the system's pointers are swapped now, so that restoring happens once.
    static SWAPPED: AtomicBool = AtomicBool::new(false);

    /// Swap the arrow and the text cursor for the microphone pointer.
    pub fn listening() {
        if SWAPPED.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Err(error) = swap() {
            log::error!("pointer: {error}");
        }
    }

    /// The system's own pointers again, if Mundwerk swapped them.
    pub fn restore() {
        if SWAPPED.swap(false, Ordering::SeqCst) {
            reset();
        }
    }

    /// The system's own pointers, read anew from the user's settings: at the start (a crashed run may have left the
    /// microphone) and at the end.
    pub fn reset() {
        SWAPPED.store(false, Ordering::SeqCst);
        // SAFETY: SPI_SETCURSORS takes no buffer.
        if let Err(error) =
            unsafe { SystemParametersInfoW(SPI_SETCURSORS, 0, None, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)) }
        {
            log::error!("pointer: restoring the system's: {error}");
        }
    }

    fn swap() -> windows::core::Result<()> {
        // SAFETY: the bitmaps are made from PIXELS (SIZE x SIZE x 4 bytes) and a mask of the same size; the icon
        // copies them, so they are deleted after; SetSystemCursor takes over (and destroys) each copy it is given.
        unsafe {
            let colour = CreateBitmap(SIZE, SIZE, 1, 32, Some(PIXELS.as_ptr().cast()));
            // All zero: every pixel is drawn, the alpha of the colour bitmap does the shape.
            let mask_bits = vec![0u8; (SIZE * SIZE / 8) as usize];
            let mask = CreateBitmap(SIZE, SIZE, 1, 1, Some(mask_bits.as_ptr().cast()));
            let info = ICONINFO { fIcon: false.into(), xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: colour };
            let icon = CreateIconIndirect(&raw const info);
            let _ = DeleteObject(HGDIOBJ(colour.0));
            let _ = DeleteObject(HGDIOBJ(mask.0));
            let icon = icon?;
            for id in [OCR_NORMAL, OCR_IBEAM] {
                SetSystemCursor(HCURSOR(CopyIcon(icon)?.0), id)?;
            }
            DestroyIcon(icon)
        }
    }
}
