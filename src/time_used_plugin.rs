use bevy::core::Time;
use bevy::{
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics},
    prelude::*,
};

#[derive(Default)]
pub struct TimeUsed(pub Time);

pub struct TimeUsedPlugin;

impl TimeUsedPlugin {
    pub const TIME_USED: DiagnosticId =
        DiagnosticId::from_u128(165001119176470494608143085728972588940);

    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(Diagnostic::new(Self::TIME_USED, "time_used", 20));
    }
    fn time_used_start(mut time: ResMut<TimeUsed>) {
        time.0.update();
    }
    fn time_used_end(mut time: ResMut<TimeUsed>, mut diagnostics: ResMut<Diagnostics>) {
        time.0.update();
        let dt = time.0.delta_seconds_f64();
        diagnostics.add_measurement(Self::TIME_USED, dt);
    }
}

impl Plugin for TimeUsedPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_stage_before(
            CoreStage::First,
            "very_first",
            SystemStage::single_threaded(),
        )
        .add_stage_after(CoreStage::Last, "very_last", SystemStage::single_threaded())
        .add_system_to_stage("very_first", Self::time_used_start.system())
        .add_system_to_stage("very_last", Self::time_used_end.system())
        .add_startup_system(Self::setup_system.system())
        .init_resource::<TimeUsed>();
    }
}
