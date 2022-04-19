use bevy::{app::AppExit, prelude::*, winit::WinitSettings};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::WorldInspectorPlugin;

#[derive(Component, Deref)]
struct ButtonPos(usize);

#[derive(Clone, Component)]
enum Action {
    Menu(Menu),
    Back,
    Quit,
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
    fn menu_at(&self, mut pos: Vec<usize>) -> &Menu {
        if pos.is_empty() {
            self
        } else {
            if let Action::Menu(menu) = &self.buttons[pos.remove(0)].action {
                menu.menu_at(pos)
            } else {
                panic!("Menu pos does not point to a menu");
            }
        }
    }

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

                for (pos, button) in self.buttons.iter().enumerate() {
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
                        .insert(ButtonPos(pos))
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

struct CurrMenu {
    menu: Menu,
    menu_e: Entity,
    pos: Vec<usize>,
}

impl CurrMenu {
    fn nav(&mut self, nav_menu: &NavMenu, commands: &mut Commands, asset_server: &AssetServer) {
        commands.entity(self.menu_e).despawn_recursive();

        match nav_menu {
            NavMenu::Fore(pos) => {
                self.pos.push(*pos);
            }
            NavMenu::Back => {
                self.pos.pop();
            }
        }

        self.menu_e = self
            .menu
            .menu_at(self.pos.clone())
            .spawn(commands, asset_server);
    }
}

#[derive(Deref)]
struct SpawnMenu(Menu);

enum NavMenu {
    Fore(usize),
    Back,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_event::<SpawnMenu>()
        .add_event::<NavMenu>()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WinitSettings::desktop_app())
        .add_startup_system(init)
        .add_system(button_action)
        .add_system(nav_menu)
        .add_system(spawn_menu);

    #[cfg(feature = "inspector")]
    app.add_plugin(WorldInspectorPlugin::new());

    app.run();
}

fn init(mut commands: Commands, mut spawn_menus: EventWriter<SpawnMenu>) {
    commands.spawn_bundle(UiCameraBundle::default());

    spawn_menus.send(SpawnMenu(Menu {
        title: "voxmod".to_string(),
        buttons: vec![
            MenuButton {
                text: "Play".to_string(),
                action: Action::Menu(Menu {
                    title: "Choose a world".to_string(),
                    buttons: vec![
                        MenuButton {
                            text: "World 1".to_string(),
                            action: Action::Quit,
                        },
                        MenuButton {
                            text: "World 2".to_string(),
                            action: Action::Quit,
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
                action: Action::Quit,
            },
            MenuButton {
                text: "Quit".to_string(),
                action: Action::Quit,
            },
        ],
    }));
}

fn spawn_menu(
    mut commands: Commands,
    mut spawn_menus: EventReader<SpawnMenu>,
    asset_server: Res<AssetServer>,
) {
    for spawn_menu in spawn_menus.iter() {
        let menu_e = spawn_menu.spawn(&mut commands, &asset_server);
        commands.insert_resource(CurrMenu {
            menu: (**spawn_menu).clone(),
            menu_e,
            pos: Vec::default(),
        })
    }
}

fn nav_menu(
    mut commands: Commands,
    mut nav_menus: EventReader<NavMenu>,
    mut curr_menu: Option<ResMut<CurrMenu>>,
    asset_server: Res<AssetServer>,
) {
    for nav_menu in nav_menus.iter() {
        if let Some(curr_menu) = &mut curr_menu {
            curr_menu.nav(nav_menu, &mut commands, &asset_server);
        } else {
            panic!("Attempted to navigate menu when no menu was open");
        }
    }
}

fn button_action(
    mut app_exits: EventWriter<AppExit>,
    mut nav_menus: EventWriter<NavMenu>,
    mut interactions: Query<
        (&Interaction, &mut UiColor, &Action, &ButtonPos),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, action, pos) in interactions.iter_mut() {
        *color = match interaction {
            Interaction::Clicked => {
                match action {
                    Action::Menu(_) => nav_menus.send(NavMenu::Fore(**pos)),
                    Action::Back => nav_menus.send(NavMenu::Back),
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
