//! Output management - simplified

use smithay::{
    output::{Output, Mode, PhysicalProperties, Subpixel},
    utils::{Rectangle, Transform},
};
use crate::AppState;

/// Create a default output
pub fn create_default_output(state: &mut AppState) -> Output {
    let output = Output::new(
        "default".to_string(),
        PhysicalProperties {
            size: (1920, 1080).into(),
            subpixel: Subpixel::Unknown,
            make: "HorizonOS".to_string(),
            model: "Graph Desktop".to_string(),
        },
    );
    
    let mode = Mode {
        size: (1920, 1080).into(),
        refresh: 60_000,
    };
    
    output.change_current_state(
        Some(mode),
        Some(Transform::Normal),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);
    
    state.space.map_output(&output, (0, 0));
    
    output
}