class_name ForceFieldDetector extends Area3D
## Detects and exposes the current force from all containing force fields

var _force_fields: Array[ForceField] = []

## The current acceleration force vector applying to this area
var acceleration: Vector3 = Vector3.DOWN * ForceField.DEFAULT_ACCELERATION
## The normalized force vector applying to this area
var down: Vector3 = Vector3.DOWN
## The opposite of the normalized force vector applying to this area
var up: Vector3 = Vector3.UP

func _physics_process(_delta: float) -> void:
	# continue to apply last known forcefield if we fall out of them
	if _force_fields.is_empty():
		return
	
	# The most recently added, highest priority forcefield will be the one used.
	var highest_priority = 0
	for force_field in _force_fields:
		if (force_field.priority >= highest_priority):
			highest_priority = force_field.priority
			acceleration = force_field.get_acceleration_at(global_transform.origin)
	
	down = acceleration.normalized()
	up = - down

# track what fields we're in
func _on_area_entered(area: Area3D) -> void:
	_force_fields.append(area)

# track what fields we're in
func _on_area_exited(area: Area3D) -> void:
	_force_fields.erase(area)
