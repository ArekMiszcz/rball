extends RigidBody2D

var new_location = Vector2()

func _ready():
	new_location = position

func _process(_delta):
	global_position = global_position.linear_interpolate(new_location, 0.4)

func _on_Network_ball_move(location):
	new_location = location;
