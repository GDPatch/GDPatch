extends Logger

func _log_message(message: String, error: bool) -> void:
  GDPatch.log_message("error" if error else "info", message)

func _log_error(function: String, file: String, line: int, code: String, rationale: String, _editor_notify: bool,
    error_type: ErrorType, _script_backtraces: Array[ScriptBacktrace]) -> void:
  GDPatch.log_error(function, file, line, code, rationale, error_type)

func register() -> void:
  OS.add_logger(self)
