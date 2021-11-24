extends KinematicBody2D

export (int) var speed = 100

signal player_move(location)
signal player_kick()

var velocity = Vector2()

var new_location = Vector2()
var moving = false

func _ready():
	new_location = position;
	scale = Vector2(0.2, 0.2);

func get_input():
	velocity = Vector2()
	if Input.is_action_just_pressed("ui_kick"):
		emit_signal("player_kick");
	if Input.is_action_pressed('ui_right'):
		velocity.x += 1
	if Input.is_action_pressed('ui_left'):
		velocity.x -= 1
	if Input.is_action_pressed('ui_down'):
		velocity.y += 1
	if Input.is_action_pressed('ui_up'):
		velocity.y -= 1
	velocity = velocity.normalized() * speed
	moving = velocity.length() != 0

func _physics_process(_delta):
	get_input()

	if moving:
		emit_signal("player_move", velocity);
		
func _process(_delta):
	position = position.linear_interpolate(new_location, 0.2)

func _on_Network_server_player_move(location):
	new_location = location
