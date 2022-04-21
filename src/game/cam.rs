use std::f32::consts::FRAC_PI_2;

use bevy::{input::mouse::MouseMotion, prelude::*, render::camera::Camera3d};

use crate::state::GameState;

pub struct CamPlugin;

impl Plugin for CamPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(GameState::Game).with_system(init_cam))
            .add_system_set(
                SystemSet::on_update(GameState::Game)
                    .with_system(look_cam)
                    .with_system(move_cam),
            )
            .add_system_set(SystemSet::on_exit(GameState::Game).with_system(exit_cam));
    }
}

#[derive(Component, Default)]
struct Rotation {
    pitch: f32,
    yaw: f32,
}

fn init_cam(mut commands: Commands, mut windows: ResMut<Windows>) {
    commands
        .spawn_bundle(PerspectiveCameraBundle::default())
        .insert(Rotation::default());

    let window = windows.primary_mut();
    window.set_cursor_lock_mode(true);
    window.set_cursor_visibility(false);
}

const MOUSE_SENSITIVITY: f32 = 0.000075;

fn look_cam(
    mut mouse_motions: EventReader<MouseMotion>,
    mut cams: Query<(&mut Rotation, &mut Transform), With<Camera3d>>,
    windows: Res<Windows>,
    time: Res<Time>,
) {
    let window = windows.primary();
    let window_scale = window.height().min(window.width());
    for mouse_motion in mouse_motions.iter() {
        for (mut rotation, mut tf) in cams.iter_mut() {
            rotation.pitch -=
                (mouse_motion.delta.y * window_scale * time.delta_seconds() * MOUSE_SENSITIVITY)
                    .clamp(-FRAC_PI_2, FRAC_PI_2);
            rotation.yaw -=
                mouse_motion.delta.x * window_scale * time.delta_seconds() * MOUSE_SENSITIVITY;

            tf.rotation = Quat::from_axis_angle(Vec3::Y, rotation.yaw)
                * Quat::from_axis_angle(Vec3::X, rotation.pitch);
        }
    }
}

const CAMERA_SPEED: f32 = 50.;

fn move_cam(
    mut cams: Query<&mut Transform, With<Camera3d>>,
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
) {
    for mut tf in cams.iter_mut() {
        let local_z = tf.local_z();

        tf.translation += ((keys.pressed(KeyCode::Comma) as i32 - keys.pressed(KeyCode::O) as i32)
            as f32
            * -Vec3::new(local_z.x, 0., local_z.z)
            + (keys.pressed(KeyCode::E) as i32 - keys.pressed(KeyCode::A) as i32) as f32
                * Vec3::new(local_z.z, 0., -local_z.x)
            + (keys.pressed(KeyCode::Space) as i32 - keys.pressed(KeyCode::LShift) as i32) as f32
                * Vec3::Y)
            .normalize_or_zero()
            * time.delta_seconds()
            * CAMERA_SPEED;
    }
}

fn exit_cam(mut windows: ResMut<Windows>) {
    let window = windows.primary_mut();
    window.set_cursor_lock_mode(false);
    window.set_cursor_visibility(true);
}
