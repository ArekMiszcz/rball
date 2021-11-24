extends Button

signal button_pressed

onready var game = get_node("/root/Game")
onready var line_edit = get_node("../LineEdit")

func _ready():
	self.connect("pressed", self, "_on_button_pressed")

func _on_button_pressed():
	emit_signal("button_pressed", line_edit.text)

func _process(_delta):
	if not line_edit.text.empty():
		self.disabled = false
	else:
		self.disabled = true
