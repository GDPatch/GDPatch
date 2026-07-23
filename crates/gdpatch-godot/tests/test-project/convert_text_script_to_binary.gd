extends MainLoop

const Harness = preload("res://harness.gd")

func _initialize() -> void:
    var buffer = Harness.read_input()

    if buffer == null:
        return

    var source = buffer.get_string_from_utf8()
    var converted = BoundGDTokenizerBuffer.parse_code_string(source)
    print(converted.hex_encode())

func _process() -> bool:
    return true

func _finalize() -> void:
    pass
