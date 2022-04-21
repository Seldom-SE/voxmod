mod game;
mod menu;
mod state;

use bevy::{app::AppExit, prelude::*};
use bevy_asset_loader::AssetLoader;
#[cfg(feature = "inspector")]
use bevy_inspector_egui::WorldInspectorPlugin;
use game::GamePlugin;
use menu::{Fonts, MenuPlugin};
use state::{GameState, OpeningGame, StatePlugin};

fn main() {
    let mut app = App::new();

    AssetLoader::new(GameState::Loading)
        .continue_to_state(GameState::MainMenu)
        .with_collection::<Fonts>()
        .build(&mut app);

    app.insert_resource(WindowDescriptor {
        // voxmod? blockmod? boxmod? boxelmod?
        title: "blockmod".to_string(),
        ..default()
    })
    .add_plugins(DefaultPlugins)
    .add_plugin(GamePlugin)
    .add_plugin(MenuPlugin)
    .add_plugin(StatePlugin)
    .insert_resource(ClearColor(Color::BLACK))
    .add_system_set(SystemSet::on_resume(GameState::MainMenu).with_system(exit));

    #[cfg(feature = "inspector")]
    app.add_plugin(WorldInspectorPlugin::new());

    app.run();
}

fn exit(
    mut commands: Commands,
    mut app_exits: EventWriter<AppExit>,
    opening_game: Option<Res<OpeningGame>>,
) {
    if opening_game.is_none() {
        app_exits.send(AppExit);
    } else {
        commands.remove_resource::<OpeningGame>();
    }
}
