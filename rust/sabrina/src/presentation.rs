// SPDX-License-Identifier: MIT
//! One interpolated state shared by models, skeletons and attached effects.
use crate::app::{SessionUi, Simulation};
use bevy::prelude::*;
use reef_sim as sim;

#[derive(Resource, Default)]
pub struct Previous(pub Option<sim::World>);
#[derive(Resource, Default)]
pub struct Presentation(pub sim::World);

pub fn prepare(
    simulation: Res<Simulation>,
    previous: Res<Previous>,
    fixed: Res<Time<Fixed>>,
    ui: Res<SessionUi>,
    mut rendered: ResMut<Presentation>,
) {
    let current = &simulation.0;
    let adjacent = previous.0.as_ref().filter(|p| {
        (p.session.time + fixed.timestep().as_secs_f32() - current.session.time).abs() < 0.00001
    });
    if let Some(previous) = adjacent {
        // Freeze the last displayed pose, rather than jumping to a tick boundary.
        if !ui.paused() {
            rendered.0 =
                sim::presentation::interpolate(previous, current, fixed.overstep_fraction());
        }
    } else {
        // Loading, retry, menu reset and deliberately frozen asset previews.
        rendered.0.clone_from(current);
    }
}
