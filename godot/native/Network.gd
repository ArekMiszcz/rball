extends Node2D

var IP_CLIENT = "127.0.0.1"
var IP_SERVER = "164.90.191.192"
var PORT_SERVER = 12350
var PORT_CLIENT = 12353

var socketUDP = PacketPeerUDP.new()

var player_name = ""

signal player_connected(name, location)
signal enemy_connected(name, location)
signal changed_player_team(name, team)
signal enemy_disconnected(name)

signal ball_move(location)
signal enemy_move(name, location)
signal server_player_move(location)

var unreliable_packet_header = [84,9,0,0,0] #delivery: Unreliable, ordering: None
var reliable_packet_header = [84,9,0,1,0,0,0,255,255,0,0,0,0] #delivery: Reliable, ordering: None

func define_player_name():
	player_name = "{ip_client}:{port_client}".format({
		"ip_client": IP_CLIENT,
		"port_client": PORT_CLIENT
	})

func try_connect():
	var recv_buf_size = 65536000;
	var response = socketUDP.listen(PORT_CLIENT, IP_CLIENT, recv_buf_size)
	if response == ERR_UNAVAILABLE:
		printt("Server unavailable on port: " + str(PORT_CLIENT) + " in server: " + IP_SERVER)
		PORT_CLIENT = PORT_CLIENT + 1
		return try_connect()
	return response

func start_client(player_nickname):
	if try_connect() != OK:
		printt("Error listening on port: " + str(PORT_CLIENT) + " in server: " + IP_SERVER)
	else:
		printt("Listening on port: " + str(PORT_CLIENT) + " in server: " + IP_SERVER)
		define_player_name()
		socketUDP.set_dest_address(IP_SERVER, PORT_SERVER)
		send_packet(JSON.print({
			"kind": "Connect", 
			"payload": JSON.print({
				"name": player_name,
				"nickname": player_nickname
			})
		}), true)

func _on_Game_connect_new_player(player_nickname):
	start_client(player_nickname)
	pass

func send_packet(body, relieble = true):
	var header
	if relieble:
		header = PoolByteArray(reliable_packet_header)
	else:
		header = PoolByteArray(unreliable_packet_header)
	for c in body:
		header.append(ord(c))
	socketUDP.put_packet(PoolByteArray(header))

func _process(_delta):
	if socketUDP.get_available_packet_count() > 0:
		var array_bytes = socketUDP.get_packet()
		for i in unreliable_packet_header.size():
			array_bytes.remove(0)
		var s = ""
		for c in array_bytes:
			s += char(c)
		var response = JSON.parse(s)
		if response.error == OK:
			var payload = JSON.parse(response.result.payload)
			if payload.error == OK:
				if payload.result.has("action"):
					if payload.result.get("action") == "PLAYER_ADD_ACK":
						var position = payload.result.get("position");
						var name = payload.result.get("name");
						var nickname = payload.result.get("nickname");
						if name == player_name:
							emit_signal(
								"player_connected",
								name,
								nickname,
								Vector2(position.get("x"), position.get("y"))
							)
						else:
							emit_signal(
								"enemy_connected",
								name,
								nickname,
								Vector2(position.get("x"), position.get("y"))
							)
					elif payload.result.get("action") == "CHANGE_PLAYER_TEAM_ACK":
						var name = payload.result.get("name");
						var team = payload.result.get("team");
						emit_signal("changed_player_team", name, team)
					elif payload.result.get("action") == "PLAYER_DISCONNECT_ACK":
						var name = payload.result.get("name");
						emit_signal("enemy_disconnected", name)
					elif payload.result.get("action") == "PLAYER_MOVED":
						var player_position = payload.result.get("position");
						emit_signal("server_player_move", Vector2(player_position.get("x"), player_position.get("y")))
					elif payload.result.get("action") == "ENEMY_MOVED":
						var name = payload.result.get("name");
						var position = payload.result.get("position");
						emit_signal("enemy_move", name, Vector2(position.get("x"), position.get("y")))
					elif payload.result.get("action") == "BALL_MOVED":
						var position = payload.result.get("position");
						emit_signal("ball_move", Vector2(position.get("x"), position.get("y")))

func _on_Game_change_team(team):
	if socketUDP.is_listening():
		var stg = JSON.print({ 
			"kind": "Data", 
			"payload": JSON.print({
				"action": "CHANGE_PLAYER_TEAM",
				"team": team
			})
		})
		send_packet(stg)

func _on_Player_player_move(location):
	if socketUDP.is_listening():
		var stg = JSON.print({ 
			"kind": "Data", 
			"payload": JSON.print({
				"action": "PLAYER_MOVED",
				"position":{
					"x":location.x,
					"y":location.y
				}
			})
		})
		send_packet(stg)

func _on_Player_player_kick():
	if socketUDP.is_listening():
		var stg = JSON.print({ 
			"kind": "Data", 
			"payload": JSON.print({
				"action": "PLAYER_KICKED"
			})
		})
		send_packet(stg)

func _exit_tree():
	if socketUDP.is_listening():
		var stg = JSON.print({ 
			"kind": "Data", 
			"payload": JSON.print({
				"action": "PLAYER_DISCONNECTED"
			})
		})
		send_packet(stg)
	socketUDP.close()
