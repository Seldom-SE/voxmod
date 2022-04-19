mod menu;
mod state;

use bevy::{app::AppExit, prelude::*};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::WorldInspectorPlugin;
use menu::MenuPlugin;
use state::{GameState, StatePlugin};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugin(StatePlugin)
        .add_plugin(MenuPlugin)
        .add_system_set(SystemSet::on_resume(GameState::MainMenu).with_system(exit));

    #[cfg(feature = "inspector")]
    app.add_plugin(WorldInspectorPlugin::new());

    app.run();
}

fn exit(mut app_exits: EventWriter<AppExit>) {
    app_exits.send(AppExit);
}
