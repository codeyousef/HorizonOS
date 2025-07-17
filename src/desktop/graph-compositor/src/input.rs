//! Input handling for the compositor - updated for Smithay 0.7

use smithay::{
    backend::input::{
        Event, InputBackend, InputEvent, KeyState, KeyboardKeyEvent,
        PointerMotionEvent, PointerButtonEvent, PointerAxisEvent,
    },
    input::{
        keyboard::{keysyms, FilterResult},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    utils::{Point, Serial, SERIAL_COUNTER},
};
use crate::AppState;

/// Process input events
pub fn process_input_event<I: InputBackend>(
    state: &mut AppState,
    event: InputEvent<I>,
) {
    match event {
        InputEvent::Keyboard { event } => {
            let serial = SERIAL_COUNTER.next_serial();
            let keyboard = state.seat.get_keyboard().unwrap();
            
            keyboard.input::<(), _>(
                state,
                event.key_code(),
                event.state(),
                serial,
                event.time_msec(),
                |state, modifiers, handle| {
                    // Check for compositor shortcuts
                    if modifiers.alt && event.state() == KeyState::Pressed {
                        if handle.modified_sym().raw() == keysyms::KEY_q {
                            // Alt+Q to quit
                            state.running = false;
                            return FilterResult::Intercept(());
                        }
                    }
                    
                    FilterResult::Forward
                },
            );
        }
        InputEvent::PointerMotion { event } => {
            let pointer = state.seat.get_pointer().unwrap();
            let delta = event.delta();
            
            // Update pointer position
            let current_location = pointer.current_location();
            let new_location = current_location + delta;
            
            pointer.motion(
                state,
                None, // TODO: Find surface under pointer
                &MotionEvent {
                    location: new_location,
                    serial: SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                },
            );
        }
        InputEvent::PointerButton { event } => {
            let pointer = state.seat.get_pointer().unwrap();
            
            pointer.button(
                state,
                &ButtonEvent {
                    button: event.button_code(),
                    state: event.state().into(),
                    serial: SERIAL_COUNTER.next_serial(),
                    time: event.time_msec(),
                },
            );
        }
        _ => {} // Handle other events as needed
    }
}