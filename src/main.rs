use bevy::{app::AppExit, prelude::*};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::WorldInspectorPlugin;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum GameState {
    MainMenu,
    Menu,
    Buffer,
}

#[derive(Deref)]
struct BufferedState(GameState);

#[derive(Clone, Component)]
enum Action {
    Menu(Menu),
    Back,
}

#[derive(Clone)]
struct MenuButton {
    text: String,
    action: Action,
}

#[derive(Clone)]
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
    fn spawn(&self, commands: &mut Commands, asset_server: &AssetServer) -> Entity {
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
                        self.title.clone(),
                        TextStyle {
                            font: font.clone(),
                            font_size: MENU_TITLE_SIZE,
                            color: MENU_TITLE_COLOR,
                        },
                        default(),
                    ),
                    ..default()
                });

                for button in &self.buttons {
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
                        .insert(button.action.clone())
                        .with_children(|parent| {
                            parent.spawn_bundle(TextBundle {
                                text: Text::with_section(
                                    button.text.clone(),
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
            })
            .id()
    }
}

#[derive(Deref, DerefMut)]
struct MenuEs(Vec<Entity>);

#[derive(Deref)]
struct NextMenu(Menu);

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::BLACK))
        .add_state(GameState::MainMenu)
        .add_system_set(SystemSet::on_enter(GameState::MainMenu).with_system(init))
        .add_system_set(SystemSet::on_resume(GameState::MainMenu).with_system(term))
        .add_system_set(SystemSet::on_enter(GameState::Menu).with_system(init_menu))
        .add_system_set(SystemSet::on_update(GameState::Menu).with_system(button_action))
        .add_system_set(SystemSet::on_exit(GameState::Menu).with_system(term_menu))
        .add_system_set(SystemSet::on_enter(GameState::Buffer).with_system(push_state))
        .add_system_set(SystemSet::on_resume(GameState::Buffer).with_system(pop_state));

    #[cfg(feature = "inspector")]
    app.add_plugin(WorldInspectorPlugin::new());

    app.run();
}

fn init(mut commands: Commands, mut state: ResMut<State<GameState>>) {
    commands.spawn_bundle(UiCameraBundle::default());

    commands.insert_resource(NextMenu(Menu {
        title: "blockmod".to_string(),
        buttons: vec![
            MenuButton {
                text: "Play".to_string(),
                action: Action::Menu(Menu {
                    title: "Choose a world".to_string(),
                    buttons: vec![
                        MenuButton {
                            text: "World 1".to_string(),
                            action: Action::Back,
                        },
                        MenuButton {
                            text: "World 2".to_string(),
                            action: Action::Back,
                        },
                        MenuButton {
                            text: "Back".to_string(),
                            action: Action::Back,
                        },
                    ],
                }),
            },
            MenuButton {
                text: "Edit".to_string(),
                action: Action::Back,
            },
            MenuButton {
                text: "Quit".to_string(),
                action: Action::Back,
            },
        ],
    }));
    state.push(GameState::Menu).unwrap();
}

fn term(mut app_exits: EventWriter<AppExit>) {
    app_exits.send(AppExit);
}

fn init_menu(
    mut commands: Commands,
    mut menu_es: Option<ResMut<MenuEs>>,
    mut nodes: Query<&mut Style, With<Node>>,
    asset_server: Res<AssetServer>,
    next_menu: Res<NextMenu>,
) {
    let menu_e = next_menu.spawn(&mut commands, &asset_server);
    if let Some(menu_es) = &mut menu_es {
        nodes.get_mut(*menu_es.last().unwrap()).unwrap().display = Display::None;
        menu_es.push(menu_e);
    } else {
        commands.insert_resource(MenuEs(vec![menu_e]));
    }

    commands.remove_resource::<NextMenu>();
}

fn button_action(
    mut commands: Commands,
    mut interactions: Query<
        (&Interaction, &mut UiColor, &Action),
        (Changed<Interaction>, With<Button>),
    >,
    mut state: ResMut<State<GameState>>,
) {
    for (interaction, mut color, action) in interactions.iter_mut() {
        *color = match interaction {
            Interaction::Clicked => {
                match action {
                    Action::Menu(menu) => {
                        commands.insert_resource(BufferedState(GameState::Menu));
                        commands.insert_resource(NextMenu(menu.clone()));
                        state.push(GameState::Buffer).unwrap();
                    }
                    Action::Back => state.pop().unwrap(),
                }
                BUTTON_PRESS_COLOR
            }
            Interaction::Hovered => BUTTON_HOVER_COLOR,
            Interaction::None => BUTTON_COLOR,
        }
        .into();
    }
}

fn term_menu(
    mut commands: Commands,
    mut nodes: Query<&mut Style, With<Node>>,
    mut menu_es: ResMut<MenuEs>,
) {
    commands.entity(menu_es.pop().unwrap()).despawn_recursive();
    if let Some(menu_e) = menu_es.last() {
        nodes.get_mut(*menu_e).unwrap().display = Display::Flex;
    } else {
        commands.remove_resource::<MenuEs>();
    }
}

fn push_state(
    mut commands: Commands,
    buffered_state: Res<BufferedState>,
    mut state: ResMut<State<GameState>>,
) {
    state.push(buffered_state.clone()).unwrap();
    commands.remove_resource::<BufferedState>();
}

fn pop_state(mut state: ResMut<State<GameState>>) {
    state.pop().unwrap();
}
