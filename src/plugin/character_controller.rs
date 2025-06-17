use avian3d::{
    math::*,
    prelude::{NarrowPhaseSet, *},
};
use bevy::{ecs::query::Has, prelude::*, time::Stopwatch};

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementAction>()
            .add_systems(
                Update,
                (
                    find_gravity_influence,
                    keyboard_input,
                    gamepad_input,
                    update_grounded,
                    apply_gravity,
                    apply_movement_damping,
                    movement,
                )
                    .chain(),
            )
            .add_systems(
                // Run collision handling after collision detection.
                //
                // NOTE: The collision implementation here is very basic and a bit buggy.
                //       A collide-and-slide algorithm would likely work better.
                PhysicsSchedule,
                kinematic_controller_collisions.in_set(NarrowPhaseSet::Last),
            );
    }
}

/// An event sent for a movement input action.
#[derive(Event)]
pub enum MovementAction {
    Move(Vector2),
    Jump,
}

/// A marker component indicating that an entity is using a character controller.
#[derive(Component)]
pub struct CharacterController;

/// A marker component indicating that an entity is on the ground.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

/// A timer componnent for tracking the turn around window for side flips
#[derive(Component)]
struct TurnAroundWindow(Stopwatch);

/// The acceleration used for character movement.
#[derive(Component)]
pub struct MovementAcceleration(Scalar);

/// The strength of a jump.
#[derive(Component)]
pub struct JumpImpulse(Scalar);

/// The gravitational acceleration.
#[derive(Component)]
pub struct GravityAcceleration(Scalar);

/// The gravitational body
#[derive(Component)]
pub struct GravityField {
    pub field_type: GravityFieldType,
    pub priority: u8,
}

pub enum GravityFieldType {
    Sphere,
    HalfPlane,
}

#[derive(Component)]
pub struct GravityDirection(Vector);

/// The maximum angle a slope can have for a character controller
/// to be able to climb and jump. If the slope is steeper than this angle,
/// the character will slide down.
#[derive(Component)]
pub struct MaxSlopeAngle(Scalar);

/// A bundle that contains the components needed for a basic
/// kinematic character controller.
#[derive(Bundle)]
pub struct CharacterControllerBundle {
    character_controller: CharacterController,
    body: RigidBody,
    collider: Collider,
    ground_caster: ShapeCaster,
    gravity: GravityAcceleration,
    movement: MovementBundle,
}

/// A bundle that contains components for character movement.
#[derive(Bundle)]
pub struct MovementBundle {
    acceleration: MovementAcceleration,
    jump_impulse: JumpImpulse,
    max_slope_angle: MaxSlopeAngle,
    turn_around_window: TurnAroundWindow,
}

impl MovementBundle {
    pub fn new(acceleration: Scalar, jump_impulse: Scalar, max_slope_angle: Scalar) -> Self {
        Self {
            acceleration: MovementAcceleration(acceleration),
            jump_impulse: JumpImpulse(jump_impulse),
            max_slope_angle: MaxSlopeAngle(max_slope_angle),
            turn_around_window: TurnAroundWindow(Stopwatch::new()),
        }
    }
}

impl Default for MovementBundle {
    fn default() -> Self {
        Self::new(30.0, 7.0, PI * 0.45)
    }
}

impl CharacterControllerBundle {
    pub fn new(collider: Collider, gravity: f32) -> Self {
        // Create shape caster as a slightly smaller version of collider
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vector::ONE * 0.99, 10);

        Self {
            character_controller: CharacterController,
            body: RigidBody::Kinematic,
            collider,
            ground_caster: ShapeCaster::new(
                caster_shape,
                Vector::ZERO,
                Quaternion::default(),
                Dir3::NEG_Y,
            )
            .with_max_distance(0.2),
            gravity: GravityAcceleration(gravity),
            movement: MovementBundle::default(),
        }
    }

    pub fn with_movement(
        mut self,
        acceleration: Scalar,
        jump_impulse: Scalar,
        max_slope_angle: Scalar,
    ) -> Self {
        self.movement = MovementBundle::new(acceleration, jump_impulse, max_slope_angle);
        self
    }
}

fn find_gravity_influence(
    mut commands: Commands,
    collisions: Collisions,
    mut bodies: Query<(Entity, &Transform), With<GravityAcceleration>>,
    gravity_fields: Query<(&GravityField, &Transform)>,
) {
    for (body, body_transform) in bodies.iter_mut() {
        let mut field_of_influence = None;
        for contacts in collisions.collisions_with(body) {
            let Ok(field) = gravity_fields
                .get(contacts.collider1)
                .or_else(|_| gravity_fields.get(contacts.collider2))
            else {
                continue;
            };
            match field_of_influence {
                None => {
                    field_of_influence = Some(field);
                }
                Some(current_field) => {
                    if current_field.0.priority < field.0.priority {
                        field_of_influence = Some(field);
                    }
                }
            }
        }

        // set the gravity
        if let Some((field, field_transform)) = field_of_influence {
            commands
                .entity(body)
                .insert(GravityDirection(match field.field_type {
                    GravityFieldType::Sphere => {
                        (field_transform.translation - body_transform.translation).normalize()
                    }
                    GravityFieldType::HalfPlane => field_transform.rotation.mul_vec3(Vector::NEG_Y),
                }));
        }
    }
}

/// Sends [`MovementAction`] events based on keyboard input.
fn keyboard_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let up = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let down = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

    let horizontal = right as i8 - left as i8;
    let vertical = up as i8 - down as i8;
    let direction = Vector2::new(horizontal as Scalar, vertical as Scalar).clamp_length_max(1.0);

    if direction != Vector2::ZERO {
        movement_event_writer.write(MovementAction::Move(direction));
    }

    if keyboard_input.just_pressed(KeyCode::Space) {
        movement_event_writer.write(MovementAction::Jump);
    }
}

/// Sends [`MovementAction`] events based on gamepad input.
fn gamepad_input(
    mut movement_event_writer: EventWriter<MovementAction>,
    gamepads: Query<&Gamepad>,
) {
    for gamepad in gamepads.iter() {
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::LeftStickX),
            gamepad.get(GamepadAxis::LeftStickY),
        ) {
            movement_event_writer.write(MovementAction::Move(
                Vector2::new(x as Scalar, y as Scalar).clamp_length_max(1.0),
            ));
        }

        if gamepad.just_pressed(GamepadButton::South) {
            movement_event_writer.write(MovementAction::Jump);
        }
    }
}

/// Updates the [`Grounded`] status for character controllers.
fn update_grounded(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &ShapeHits,
            &Rotation,
            &GravityDirection,
            Option<&MaxSlopeAngle>,
        ),
        With<CharacterController>,
    >,
) {
    for (entity, hits, rotation, gravity_direction, max_slope_angle) in &mut query {
        // The character is grounded if the shape caster has a hit with a normal
        // that isn't too steep.
        let is_grounded = hits.iter().any(|hit| {
            if let Some(angle) = max_slope_angle {
                (rotation * hit.normal2)
                    .angle_between(gravity_direction.0)
                    .abs()
                    <= angle.0
            } else {
                true
            }
        });

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

/// Responds to [`MovementAction`] events and moves character controllers accordingly.
fn movement(
    time: Res<Time>,
    mut movement_event_reader: EventReader<MovementAction>,
    mut controllers: Query<(
        &MovementAcceleration,
        &JumpImpulse,
        &mut LinearVelocity,
        &Transform,
        &mut TurnAroundWindow,
        Has<Grounded>,
    )>,
) {
    // Precision is adjusted so that the example works with
    // both the `f32` and `f64` features. Otherwise you don't need this.
    let delta_time = time.delta_secs_f64().adjust_precision();

    for (
        movement_acceleration,
        jump_impulse,
        mut linear_velocity,
        transform,
        mut turn_around_window,
        is_grounded,
    ) in &mut controllers
    {
        // tick timing window timers
        turn_around_window.0.tick(time.delta());

        // handle movement with relation to the current player rotation in 3d space
        let mut local_linear_velocity = transform.rotation.inverse().mul_vec3(linear_velocity.0);
        for event in movement_event_reader.read() {
            let mut acceleration = movement_acceleration.0;
            if !is_grounded {
                acceleration *= 0.1;
            }
            match event {
                MovementAction::Move(direction) => {
                    let movement = Vec2 {
                        x: direction.x,
                        y: -direction.y,
                    };
                    // instant turn around on ground
                    let horizontal_speed = local_linear_velocity.xz().length();
                    if is_grounded
                        && (movement.dot(local_linear_velocity.xz()) / horizontal_speed) < -0.25
                    {
                        local_linear_velocity.x = movement.x * horizontal_speed;
                        local_linear_velocity.z = movement.y * horizontal_speed;
                        turn_around_window.0.reset();
                    } else {
                        local_linear_velocity.x += movement.x * acceleration * delta_time;
                        local_linear_velocity.z += movement.y * acceleration * delta_time;
                    }
                }
                MovementAction::Jump => {
                    if is_grounded {
                        // perform a backflip if jumping within turnaround window
                        if turn_around_window.0.elapsed_secs() < 0.2 {
                            local_linear_velocity.y += jump_impulse.0 * 1.5;
                        } else {
                            local_linear_velocity.y += jump_impulse.0;
                        }
                    }
                }
            }
        }
        // If grounded and going past top speed, apply a negative acceleration to slow back down
        if is_grounded {
            let clamped_horizontal_velocity = local_linear_velocity.xz().clamp_length_max(5.0);
            local_linear_velocity.x = clamped_horizontal_velocity.x;
            local_linear_velocity.z = clamped_horizontal_velocity.y;
        }

        linear_velocity.0 = transform.rotation.mul_vec3(local_linear_velocity);
    }
}

fn apply_gravity(
    time: Res<Time>,
    mut bodies: Query<(
        &GravityAcceleration,
        &GravityDirection,
        &mut LinearVelocity,
        &mut Transform,
    )>,
) {
    let delta_time = time.delta_secs_f64().adjust_precision();

    for (gravity_acceleration, gravity_direction, mut linear_velocity, mut transform) in &mut bodies
    {
        linear_velocity.0 += gravity_direction.0 * gravity_acceleration.0 * delta_time;

        // Smoothly rotate the player so that their up (-Y) matches the gravity direction
        let current_up = transform.rotation.mul_vec3(Vector::NEG_Y);
        let target_up = gravity_direction.0;

        // Find the rotation that aligns current_up to target_up
        let rotation_to_gravity = Quat::from_rotation_arc(current_up, target_up);

        // Compose with current rotation to get the target rotation
        let target_rotation = rotation_to_gravity * transform.rotation;

        // Slerp from current to target for smoothness
        let slerp_speed = 20.0;
        transform.rotation = transform
            .rotation
            .slerp(target_rotation, (delta_time as f32) * slerp_speed);
    }
}

/// Slows down movement in the air and on the ground
fn apply_movement_damping(
    time: Res<Time>,
    mut query: Query<(&LinearDamping, &mut LinearVelocity), With<CharacterController>>,
) {
    let delta_time = time.delta_secs_f64().adjust_precision();
    for (lin_damping, mut linear_velocity) in &mut query {
        if linear_velocity.0 != Vector::ZERO {
            linear_velocity.0 *= 1.0 / (1.0 + delta_time * lin_damping.0);
        }
    }
}

/// Kinematic bodies do not get pushed by collisions by default,
/// so it needs to be done manually.
///
/// This system handles collision response for kinematic character controllers
/// by pushing them along their contact normals by the current penetration depth,
/// and applying velocity corrections in order to snap to slopes, slide along walls,
/// and predict collisions using speculative contacts.
#[allow(clippy::type_complexity)]
fn kinematic_controller_collisions(
    collisions: Collisions,
    bodies: Query<&RigidBody>,
    collider_rbs: Query<&ColliderOf, Without<Sensor>>,
    mut character_controllers: Query<
        (&mut Position, &mut LinearVelocity),
        (With<RigidBody>, With<CharacterController>),
    >,
    time: Res<Time>,
) {
    // Iterate through collisions and move the kinematic body to resolve penetration
    for contacts in collisions.iter() {
        // Get the rigid body entities of the colliders (colliders could be children)
        let Ok([&ColliderOf { body: rb1 }, &ColliderOf { body: rb2 }]) =
            collider_rbs.get_many([contacts.collider1, contacts.collider2])
        else {
            continue;
        };

        // Get the body of the character controller and whether it is the first
        // or second entity in the collision.
        let is_first: bool;

        let character_rb: RigidBody;
        let is_other_dynamic: bool;

        let (mut position, mut linear_velocity) =
            if let Ok(character) = character_controllers.get_mut(rb1) {
                is_first = true;
                character_rb = *bodies.get(rb1).unwrap();
                is_other_dynamic = bodies.get(rb2).is_ok_and(|rb| rb.is_dynamic());
                character
            } else if let Ok(character) = character_controllers.get_mut(rb2) {
                is_first = false;
                character_rb = *bodies.get(rb2).unwrap();
                is_other_dynamic = bodies.get(rb1).is_ok_and(|rb| rb.is_dynamic());
                character
            } else {
                continue;
            };

        // This system only handles collision response for kinematic character controllers.
        if !character_rb.is_kinematic() {
            continue;
        }

        // Iterate through contact manifolds and their contacts.
        // Each contact in a single manifold shares the same contact normal.
        for manifold in contacts.manifolds.iter() {
            let normal = if is_first {
                -manifold.normal
            } else {
                manifold.normal
            };

            let mut deepest_penetration: Scalar = Scalar::MIN;

            // Solve each penetrating contact in the manifold.
            for contact in manifold.points.iter() {
                if contact.penetration > 0.0 {
                    position.0 += normal * contact.penetration;
                }
                deepest_penetration = deepest_penetration.max(contact.penetration);
            }

            // For now, this system only handles velocity corrections for collisions against static geometry.
            if is_other_dynamic {
                continue;
            }

            if deepest_penetration > 0.0 {
                // Don't apply an impulse if the character is moving away from the surface.
                if linear_velocity.dot(normal) > 0.0 {
                    continue;
                }

                // Slide along the surface, rejecting the velocity along the contact normal.
                let impulse = linear_velocity.reject_from_normalized(normal);
                linear_velocity.0 = impulse;
            } else {
                // The character is not yet intersecting the other object,
                // but the narrow phase detected a speculative collision.
                //
                // We need to push back the part of the velocity
                // that would cause penetration within the next frame.

                let normal_speed = linear_velocity.dot(normal);

                // Don't apply an impulse if the character is moving away from the surface.
                if normal_speed > 0.0 {
                    continue;
                }

                // Compute the impulse to apply.
                let impulse_magnitude =
                    normal_speed - (deepest_penetration / time.delta_secs_f64().adjust_precision());
                let impulse = impulse_magnitude * normal;
                linear_velocity.0 -= impulse;
            }
        }
    }
}
