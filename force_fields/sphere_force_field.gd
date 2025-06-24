class_name SphereForceField extends ForceField
## A force field that accelerates torwards it's center point

@export_range(0.0, 100.0, 0.01, "suffix:m/s^2", "or_greater") var acceleration = DEFAULT_ACCELERATION

func get_acceleration_at(input_position: Vector3) -> Vector3:
	return (global_transform.origin - input_position).normalized() * acceleration