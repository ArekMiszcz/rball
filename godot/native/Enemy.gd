extends KinematicBody2D

var new_location = Vector2()
var network_name = String()

func _ready():
	new_location = position
	scale = Vector2(0.2, 0.2);

func _process(_delta):
	position = position.linear_interpolate(new_location, 0.2)

func _on_Network_enemy_move(name, location):
	if network_name == name:
		new_location = location;
