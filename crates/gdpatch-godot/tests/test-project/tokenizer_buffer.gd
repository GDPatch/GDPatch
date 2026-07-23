extends MainLoop

const Harness = preload("res://harness.gd")

func _initialize() -> void:
    var buffer = Harness.read_input()

    if buffer == null:
        return

    var tokenizer = BoundGDTokenizerBuffer.new()
    tokenizer.set_code_buffer(buffer)

    var tokens = []

    while true:
        var token = tokenizer.scan()
        tokens.append({ "name": token.name, "lit": token.literal })

        if token.name == "End of file":
            break

    # emit tokens as JSON
    Harness.emit(tokens)

func _process() -> bool:
    return true

func _finalize() -> void:
    pass
