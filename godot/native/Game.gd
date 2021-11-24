extends Node2D

signal connect_new_player
signal player_connected
signal enemy_connected
signal change_team

onready var player_script = preload("res://native/Player.gd")
onready var enemy_script = preload("res://native/Enemy.gd")

onready var introduce_scene = get_node("Introduce")
onready var introduce_scene_button = get_node("Introduce/Panel/Button")

onready var network_scene = get_node("Network")

func _ready():
	introduce_scene_button.connect("button_pressed", self, "_on_Introduce_button_pressed")
	pass
	
func _on_Introduce_button_pressed(nickname):
	self.emit_signal("connect_new_player", nickname)

func _on_Network_player_connected(name, nickname, location):
	# Remove introduction scene
	self.remove_child(introduce_scene)

	# Add team manager scene
	var team_manager_scene = load("res://scenes/TeamManager.tscn")
	var team_manager = team_manager_scene.instance()
	team_manager.script = load("res://native/TeamManager.gd")
	self.connect("player_connected", team_manager, "_on_Game_player_connected")
	team_manager.connect("change_team", self, "_on_TeamManager_change_team")
	team_manager.connect("start_button_pressed", self, "_on_TeamManager_start_button_pressed")
	network_scene.connect("changed_player_team", team_manager, "_on_Network_changed_player_team")
	self.add_child(team_manager)

	emit_signal("player_connected", name, nickname)

func add_enemy(name, location):
	var enemy_scene = load("res://scenes/Enemy.tscn")
	var enemy_scene_instance = enemy_scene.instance()
	enemy_scene_instance.set_name(name)
	enemy_scene_instance.script = enemy_script
	enemy_scene_instance.position = location
	network_scene.connect("enemy_move", enemy_scene_instance, "_on_Network_enemy_move")
	add_child(enemy_scene_instance)
	get_node(enemy_scene_instance.get_name()).network_name = name

func _on_Network_enemy_connected(name, nickname, location):
	if self.has_node("TeamManager"):
		# Add enemy to team manager scene
		var team_manager = self.get_node("TeamManager")
		self.connect("enemy_connected", team_manager, "_on_Game_enemy_connected")
		emit_signal("enemy_connected", name, nickname)
	else:
		# Join enemy to the field
		add_enemy(name, location)

func _on_TeamManager_change_team(team):
	emit_signal("change_team", team)

func _on_TeamManager_start_button_pressed(player, enemies):
	#######################################################
	# Add player
	#######################################################
	var player_scene = load("res://scenes/Player.tscn")
	var player_scene_instance = player_scene.instance()
	player_scene_instance.set_name(player.network_name)
	player_scene_instance.script = player_script
	player_scene_instance.position = player.location
	player_scene_instance.connect("player_move", network_scene, "_on_Player_player_move")
	player_scene_instance.connect("player_kick", network_scene, "_on_Player_player_kick")
	network_scene.connect("server_player_move", player_scene_instance, "_on_Network_server_player_move")
	add_child(player_scene_instance)
	
	#######################################################
	# Add enemies
	#######################################################
	var enemy_scene = load("res://scenes/Enemy.tscn")
	for enemy in enemies:
		add_enemy(enemy.network_name, enemy.location)

	self.remove_child(get_node("TeamManager"))

func _on_Network_enemy_disconnected(name):
	for child in get_children():
		if not child.get("network_name") == null and child.network_name == name:
			remove_child(child)
