use bevy::{ecs::schedule::StateError, prelude::*};

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state(GameState::Loading)
            .add_system_set(SystemSet::on_enter(GameState::Buffer).with_system(push_state))
            .add_system_set(SystemSet::on_resume(GameState::Buffer).with_system(pop_state));
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum GameState {
    Loading,
    MainMenu,
    Menu,
    Buffer,
    Game,
}

#[derive(Deref)]
pub struct BufferedState(pub GameState);

pub struct OpeningGame;

fn push_state(
    mut commands: Commands,
    buffered_state: Res<BufferedState>,
    mut state: ResMut<State<GameState>>,
) {
    state.push(buffered_state.clone()).unwrap();
    commands.remove_resource::<BufferedState>();
}

#[allow(unused_must_use)]
fn pop_state(mut state: ResMut<State<GameState>>) {
    if let Err(err) = state.pop() {
        if let StateError::StateAlreadyQueued = err {
        } else {
            panic!("{}", err);
        }
    }
}
