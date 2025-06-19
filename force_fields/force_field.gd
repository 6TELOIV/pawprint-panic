class_name ForceField extends Area3D
## Base class for other force fields. Do not instantiate directly.

const DEFAULT_ACCELERATION = 19.62

## Gets the acceleration vector in this field at a global position
func get_acceleration_at(_input_position: Vector3) -> Vector3:
	return Vector3.ZERO
