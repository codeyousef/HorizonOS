//! Window management - simplified

use smithay::desktop::Window;
use crate::AppState;

/// Window-related utilities
pub fn focus_window(state: &mut AppState, window: &Window) {
    state.space.raise_element(window, true);
    
    if let Some(toplevel) = window.toplevel() {
        let surface = toplevel.wl_surface();
        state.seat.get_keyboard().unwrap().set_focus(
            state,
            Some(surface.clone()),
            smithay::utils::SERIAL_COUNTER.next_serial(),
        );
    }
}