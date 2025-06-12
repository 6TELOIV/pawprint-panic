pub struct CameraControllerPlugin;
use avian3d::math::{FRAC_PI_2, Vector3};
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementAction>()
            .add_systems(Startup, startup)
            .add_systems(Update, (mouse_input, gamepad_input, movement).chain());
    }
}

/// An event sent for a movement input action. Represents panning (XY) and zooming (Z)
#[derive(Event)]
pub enum MovementAction {
    Move(Vector3),
}

/// A marker component indicating that an entity is using a camera controller.
#[derive(Component)]
pub struct CameraController;

fn startup(mut q_windows: Query<&mut Window, With<PrimaryWindow>>) -> Result {
    let mut primary_window = q_windows.single_mut()?;
    primary_window.cursor_options = CursorOptions {
        grab_mode: CursorGrabMode::Locked,
        visible: false,
        ..default()
    };
    Ok(())
}

fn mouse_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    let movement = mouse_motion.delta.extend(mouse_scroll.delta.y);
    if movement != Vec3::ZERO {
        movement_event_writer.write(MovementAction::Move(movement));
    }
}

fn gamepad_input(
    time: Res<Time>,
    mut movement_event_writer: EventWriter<MovementAction>,
    gamepads: Query<&Gamepad>,
) {
    for gamepad in gamepads.iter() {
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::RightStickX),
            gamepad.get(GamepadAxis::RightStickY),
        ) {
            let mut movement = Vec3 { x: x, y: y, z: 0.0 };
            if let (Some(up), Some(down)) = (
                gamepad.get(GamepadButton::DPadUp),
                gamepad.get(GamepadButton::DPadDown),
            ) {
                movement.z += up - down;
            }
            if movement != Vec3::ZERO {
                movement_event_writer.write(MovementAction::Move(movement * time.delta_secs()));
            }
        }
    }
}

fn movement(
    mut movement_event_reader: EventReader<MovementAction>,
    mut camera: Single<&mut Transform, With<CameraController>>,
) {
    for event in movement_event_reader.read() {
        match event {
            MovementAction::Move(movement) => {
                let target = Vec3::ZERO;
                let distance_to_target = camera.translation.distance(target);

                // Obtain the existing pitch, yaw, and roll values from the transform.
                let (yaw, pitch, roll) = camera.rotation.to_euler(EulerRot::YXZ);

                // Establish the new yaw and pitch, preventing the pitch value from exceeding our limits.
                let pitch =
                    (pitch + movement.y * 0.003).clamp(-(FRAC_PI_2 - 0.01), FRAC_PI_2 - 0.01);
                let yaw = yaw + movement.x * 0.003;
                camera.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

                camera.translation = target - camera.forward() * (distance_to_target - movement.z);
            }
        }
    }
}
