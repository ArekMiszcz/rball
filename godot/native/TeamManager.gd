extends PanelContainer

signal change_team
signal start_button_pressed

onready var redTeamContainer = self.get_node("VBoxContainer/HBoxContainer/RedTeamContainer/ScrollContainer/RedTeam")
onready var specContainer = self.get_node("VBoxContainer/HBoxContainer/SpecContainer/ScrollContainer/SpecTeam")
onready var blueTeamContainer = self.get_node("VBoxContainer/HBoxContainer/BlueTeamContainer/ScrollContainer/BlueTeam")
onready var start_button = self.get_node("VBoxContainer/StartButton")

class Player extends Label:
	var network_name: String
	var team: String
	var location: Vector2 = Vector2(250.0, 250.0)

var global_player: Player

func _ready():
	start_button.connect("button_pressed", self, "_on_button_pressed")

func get_all_players():
	var teams = [
		redTeamContainer,
		specContainer,
		blueTeamContainer
	]
	var players = []
	for team in teams:
		var team_players = team.get_children()
		for player in team_players:
			if player and not player.network_name == global_player.network_name:
				player.team = team.name
				players.append(player)
	return players

func _on_button_pressed():
	emit_signal("start_button_pressed", global_player, get_all_players())
	
func find_player_in_the_team(container, name):
	for player in container.get_children():
		if player.network_name == name:
			return player
	return false

func assign_player_to_the_team(direction: String):
	if find_player_in_the_team(redTeamContainer, global_player.network_name):
		match direction:
			'right':
				redTeamContainer.remove_child(global_player)
				specContainer.add_child(global_player)
				emit_signal("change_team", specContainer.name)
			'left':
				return
	elif find_player_in_the_team(specContainer, global_player.network_name):
		specContainer.remove_child(global_player)
		match direction:
			'right':
				blueTeamContainer.add_child(global_player)
				emit_signal("change_team", blueTeamContainer.name)
			'left':
				redTeamContainer.add_child(global_player)
				emit_signal("change_team", redTeamContainer.name)
	elif find_player_in_the_team(blueTeamContainer, global_player.network_name):
		match direction:
			'right':
				return
			'left':
				blueTeamContainer.remove_child(global_player)
				specContainer.add_child(global_player)
				emit_signal("change_team", specContainer.name)
	pass

func _physics_process(_delta):
	if Input.is_action_just_pressed('ui_right'):
		assign_player_to_the_team('right')
	if Input.is_action_just_pressed('ui_left'):
		assign_player_to_the_team('left')
	pass

func get_team_by_name(name):
	match name:
		"RedTeam":
			return redTeamContainer
		"SpecTeam":
			return specContainer
		"BlueTeam":
			return blueTeamContainer
	return null

func switch_player_to_the_team(name, new_team):
	var teams = [
		redTeamContainer,
		specContainer,
		blueTeamContainer
	]
	for team in teams:
		var player = find_player_in_the_team(team, name)
		if player:
			team.remove_child(player)
			new_team.add_child(player)

func _on_Network_changed_player_team(name, team):
	var new_team = get_team_by_name(team)
	if new_team:
		switch_player_to_the_team(name, new_team)

func _on_Game_player_connected(name, nickname):
	global_player = Player.new()
	global_player.network_name = name
	global_player.text = nickname
	global_player.align = VALIGN_CENTER
	specContainer.add_child(global_player)
	pass

func _on_Game_enemy_connected(name, nickname):
	var enemy = Player.new()
	enemy.network_name = name
	enemy.text = nickname
	enemy.align = VALIGN_CENTER
	specContainer.add_child(enemy)
	pass
