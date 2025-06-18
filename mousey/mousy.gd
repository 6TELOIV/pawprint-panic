extends MeshInstance3D

var hue = 0.0;

# Called when the node enters the scene tree for the first time.
func _ready() -> void:
	pass # Replace with function body.


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta: float) -> void:
	hue = wrapf(hue + delta, 0, 1.0)
	mesh.surface_get_material(0).set("albedo_color", Color.from_ok_hsl(hue, 1.0, 0.5))
