extends CharacterBody3D

@export var speed := 8.0
@export var jump_strength := 10.0

## Whether or not the player is in the air from a jump
var _jumping = false
## How many jumps have been made in this sequence
var _jump_count = 0
## `true` if a jump is buffered that should be processed when the player contacts the ground
var _jump_buffered = false
## `true` if the player is currently able to jump
var _can_jump = false


@export var velocity_control_floor := 50.0
@export var velocity_control_air := 5.0

@onready var _force_field_detector: ForceFieldDetector = $ForceFieldDetector
@onready var _camera_anchor: CameraAnchor = $CameraAnchor

@onready var _coyote_timer: Timer = $CoyoteTimer
@onready var _ground_timer: Timer = $GroundTimer
@onready var _jump_buffer_timer: Timer = $JumpBufferTimer


static func get_movement_input() -> Vector2:
	var vector := Vector2(
		Input.get_action_strength("right") - Input.get_action_strength("left"),
		Input.get_action_strength("back") - Input.get_action_strength("forward")
	)
	if vector.length_squared() > 1:
		return vector.normalized() # e.g. keyboard input
	else:
		return vector


static func project_movement_intention(camera_basis: Basis, up: Vector3, movement_input: Vector2) -> Vector3:
	if movement_input == Vector2.ZERO:
		return Vector3.ZERO

	movement_input = movement_input.normalized() * min(movement_input.length(), 1)

	var up_surface = - up.cross(camera_basis.x).normalized()
	var right_surface = - up.cross(camera_basis.y).normalized()
	
	return up_surface * movement_input.y + right_surface * movement_input.x


func _physics_process(delta: float) -> void:
	# Update where we are
	_camera_anchor.target_origin = _force_field_detector.global_transform.origin
		
	var acceleration := _force_field_detector.acceleration
	if acceleration == Vector3.ZERO:
		# Weeee floating in free space!
		move_and_slide()
		return
	
	# Because we have it, update where up / down is
	_camera_anchor.target_down = _force_field_detector.down
	set_up_direction(_force_field_detector.up)

	# What controls is the player inputting?
	var movement_input := get_movement_input()

	# Where does the player want to move?
	var movement_intention := project_movement_intention(
		get_viewport().get_camera_3d().global_transform.basis,
		_force_field_detector.up,
		movement_input
	)

	# How much control does the player get over the character?
	var current_velocity_control: float
	if is_on_floor():
		current_velocity_control = velocity_control_floor
	else:
		current_velocity_control = velocity_control_air
	
	# Jumping
	_process_jumping()
	
	# Walking
	_process_walking(movement_intention, current_velocity_control * delta)
	
	# Forces acting on us
	velocity += acceleration * delta
	
	# Finally, we move!
	move_and_slide()
	
	# ... and turn
	_process_turning(delta)


func _process_jumping():
	var up = _force_field_detector.up
	if is_on_floor() || is_on_wall():
		# Player can always jump on the floor or walls
		_can_jump = true
		# If grounded and in a multi jump sequence, kick off the timer
		if _jump_count > 0 and _ground_timer.is_stopped():
			_ground_timer.start()
	else:
		# Kick off coyote timer when we leave the ground
		if _coyote_timer.is_stopped() && _can_jump:
			_coyote_timer.start()
	
	if is_on_wall():
		# hitting a wall resets your multi jump
		_jump_count = 0

	# Buffer the jump input to allow for slightly early inputs
	if !_jump_buffered && Input.is_action_just_pressed("jump"):
		_jump_buffered = true
		_jump_buffer_timer.start()

	# If the player can jump and is trying to jump, jump
	if _can_jump && _jump_buffered:
		# Clear the input
		_jump_buffered = false

		# Can't jump until we next land
		_can_jump = false

		# Cancel the jump timers; the player has jumped so they shouldn't run until the player next lands
		_ground_timer.stop()
		_jump_buffer_timer.stop()
		_coyote_timer.stop()

		# We're now jumping
		_jumping = true

		# Jump always starts with neutral upwards velocity
		velocity += -velocity.project(up)

		if is_on_wall():
			# This is a wall jump
			velocity += (up + get_wall_normal()) * jump_strength
		else:
			# Base jump impulse is modified for double and triple jumps
			var delta_v = up * jump_strength
			if _jump_count == 1:
				delta_v *= 1.25
			if _jump_count == 2:
				delta_v *= 1.5

			# Apply the impulse
			velocity += delta_v

			# Count for double and triple jumps
			_jump_count = (_jump_count + 1) % 3


func _on_coyote_timer_timeout():
	_can_jump = false


func _on_ground_timer_timeout():
	_jump_count = 0


func _on_jump_buffer_timer_timeout():
	_jump_buffered = false


func _process_walking(movement_intention: Vector3, control: float):
	var up := _force_field_detector.up

	
	var desired_velocity_change := movement_intention * speed - velocity
	# Just in case: delete "up" component.
	# If we didn't do this, we would stay levitating in the air.
	desired_velocity_change -= desired_velocity_change.project(up)

	velocity = velocity.move_toward(
		velocity + desired_velocity_change,
		control
	)


func _process_turning(delta: float):
	var up := _force_field_detector.up
	var forward := velocity;
	if (forward - forward.project(up)).is_equal_approx(Vector3.ZERO):
		forward = - transform.basis.z
	
	# Turn the player parallel to the floor and always facing their "horizontal" velocity (orthoganol to up)
	transform = Transform3D(
		transform.basis.slerp(Basis.looking_at(forward - forward.project(up), up), 20 * delta).orthonormalized(),
		transform.origin
	)
