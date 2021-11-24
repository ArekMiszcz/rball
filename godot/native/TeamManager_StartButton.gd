extends Button

signal button_pressed

func _ready():
	self.connect("pressed", self, "_on_button_pressed")

func _on_button_pressed():
	emit_signal("button_pressed")
