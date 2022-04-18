use bevy::{app::AppExit, prelude::*, winit::WinitSettings};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::WorldInspectorPlugin;

#[derive(Component)]
enum Action {
    Quit,
}

struct MenuButton {
    text: String,
    action: Action,
}

struct Menu {
    title: String,
    buttons: Vec<MenuButton>,
}

static FONT_PATH: &str = "fonts/FiraSans-Bold.ttf";
const MENU_ITEM_MARGIN: Rect<Val> = Rect {
    left: Val::Percent(0.),
    right: Val::Percent(0.),
    top: Val::Px(10.),
    bottom: Val::Px(10.),
};
const MENU_TITLE_SIZE: f32 = 100.;
const MENU_TITLE_COLOR: Color = Color::WHITE;
const BUTTON_SIZE: Size<Val> = Size {
    width: Val::Percent(50.),
    height: Val::Px(50.),
};
const BUTTON_COLOR: Color = Color::WHITE;
const BUTTON_HOVER_COLOR: Color = Color::rgb(0.75, 0.75, 0.75);
const BUTTON_PRESS_COLOR: Color = Color::GRAY;
const BUTTON_TEXT_SIZE: f32 = 50.;
const BUTTON_TEXT_COLOR: Color = Color::BLACK;

impl Menu {
    fn spawn(self, commands: &mut Commands, asset_server: &AssetServer) {
        commands
            .spawn_bundle(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::ColumnReverse,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    size: Size::new(Val::Percent(100.), Val::Percent(100.)),
                    ..default()
                },
                color: Color::NONE.into(),
                ..default()
            })
            .with_children(|parent| {
                let font = asset_server.load(FONT_PATH);

                parent.spawn_bundle(TextBundle {
                    style: Style {
                        margin: MENU_ITEM_MARGIN.clone(),
                        ..default()
                    },
                    text: Text::with_section(
                        self.title,
                        TextStyle {
                            font: font.clone(),
                            font_size: MENU_TITLE_SIZE,
                            color: MENU_TITLE_COLOR,
                        },
                        default(),
                    ),
                    ..default()
                });

                for button in self.buttons {
                    parent
                        .spawn_bundle(ButtonBundle {
                            style: Style {
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                margin: MENU_ITEM_MARGIN.clone(),
                                size: BUTTON_SIZE.clone(),
                                ..default()
                            },
                            color: BUTTON_COLOR.into(),
                            ..default()
                        })
                        .insert(button.action)
                        .with_children(|parent| {
                            parent.spawn_bundle(TextBundle {
                                text: Text::with_section(
                                    button.text,
                                    TextStyle {
                                        font: font.clone(),
                                        font_size: BUTTON_TEXT_SIZE,
                                        color: BUTTON_TEXT_COLOR,
                                    },
                                    default(),
                                ),
                                ..default()
                            });
                        });
                }
            });
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WinitSettings::desktop_app())
        .add_startup_system(init)
        .add_system(ui_action);

    #[cfg(feature = "inspector")]
    app.add_plugin(WorldInspectorPlugin::new());

    app.run();
}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(UiCameraBundle::default());

    Menu {
        title: "voxmod".to_string(),
        buttons: vec![
            MenuButton {
                text: "Play".to_string(),
                action: Action::Quit,
            },
            MenuButton {
                text: "Edit".to_string(),
                action: Action::Quit,
            },
            MenuButton {
                text: "Quit".to_string(),
                action: Action::Quit,
            },
        ],
    }
    .spawn(&mut commands, &asset_server);
}

fn ui_action(
    mut app_exits: EventWriter<AppExit>,
    mut interactions: Query<
        (&Interaction, &mut UiColor, &Action),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, action) in interactions.iter_mut() {
        *color = match interaction {
            Interaction::Clicked => {
                match action {
                    Action::Quit => app_exits.send(AppExit),
                }
                BUTTON_PRESS_COLOR
            }
            Interaction::Hovered => BUTTON_HOVER_COLOR,
            Interaction::None => BUTTON_COLOR,
        }
        .into();
    }
}
