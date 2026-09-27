extends Node

const MOD_ROOT: String = "res://gdpatch/mods/"
const BUILTIN_MOD: String = "gdpatch"
var mod_instances = {}
var mutex: Mutex
var file: FileAccess
var seq = 0
var mods: Array = []
var root_directory: String = ""

func _init() -> void:
  self.mutex = Mutex.new()
  self.file = FileAccess.open("gdpatch-ipc", FileAccess.READ_WRITE)

  self._register_script_logger()

  var mod_list = self._send_command_with_response({
    "type": "GetModList"
  })
  self.mods.assign(mod_list.value)

  var root_directory = self._send_command_with_response({
    "type": "GetRootDirectory"
  })
  self.root_directory = root_directory.value

func _ready() -> void:
  for mod in self.mods:
    var mod_id = mod["id"]
    if mod_id != BUILTIN_MOD:
      self._load_mod(mod_id)

func _register_script_logger() -> void:
  var version := Engine.get_version_info()

  if version.major == 4 and version.minor >= 5:
    var script := GDScript.new()

    script.source_code = _LOGGER

    if script.reload() == OK:
      var logger = script.new()

      # Hiding `OS.add_logger(...)` inside this dynamic script method means the parser won't fail on older Godot
      # versions that are missing `OS.add_logger(...)`.
      logger.register()

func _send_command_with_response(req):
  var this_seq = seq
  req["seq"] = this_seq
  seq = seq + 1
  self._send_command(req)

  # FIXME: broken filesilly write will make games spin forever with this loop
  while true:
    # TODO: cache seq if we receive out of order somehow?
    var resp = self._read_response()
    if resp["seq"] == this_seq: return resp

func _send_command(req):
  var str = JSON.stringify(req)
  self.mutex.lock()
  self.file.store_line(str + "\n")
  self.file.flush()
  self.mutex.unlock()

func _read_response():
  self.mutex.lock()
  var str = self.file.get_line()
  self.file.flush()
  self.mutex.unlock()
  if str == "": return
  var obj = JSON.parse_string(str)
  return obj

func _load_mod(mod_id: String) -> Node:
  var scene_path = MOD_ROOT + mod_id + "/mod.tscn"
  if FileAccess.file_exists(scene_path):
    var scene = load(scene_path)
    var node = scene.instantiate()

    node.name = mod_id

    self.add_child(node)
    mod_instances[mod_id] = node

    # print("Loaded mod " + mod_id + " as scene")
    return node

  var script_path = MOD_ROOT + mod_id + "/mod.gd"
  if FileAccess.file_exists(script_path):
    var script = load(script_path)
    var node = Node.new()

    node.name = mod_id
    node.set_script(script)

    self.add_child(node)
    mod_instances[mod_id] = node

    # print("Loaded mod " + mod_id + " as script")
    return node

  var binary_script_path = MOD_ROOT + mod_id + "/mod.gdc"
  if FileAccess.file_exists(binary_script_path):
    var script = load(binary_script_path)
    var node = Node.new()

    node.name = mod_id
    node.set_script(script)

    self.add_child(node)
    mod_instances[mod_id] = node

    # print("Loaded mod " + mod_id + " as binary script")
    return node

  # printerr("No files available for mod " + mod_id + "!")
  return

func get_mod(mod_id: String) -> Node:
  return self.mod_instances[mod_id]

func get_mods() -> Array:
  return self.mods

func get_root_directory() -> String:
  return self.root_directory

func get_mod_directory(mod_id: String):
  var resp = self._send_command_with_response({
    "type": "GetModDirectory",
    "mod_id": mod_id
  })
  return resp["value"]

func get_config_option(mod_id: String, section: String, option: String):
  var resp = self._send_command_with_response({
    "type": "GetConfigOption",
    "mod_id": mod_id,
    "section": section,
    "option": option
  })
  return resp["value"]

func set_config_option(mod_id: String, section: String, option: String, value):
  self._send_command({
    "type": "SetConfigOption",
    "mod_id": mod_id,
    "section": section,
    "option": option,
    "value": value
  })

func log_message(level: String, message: String) -> void:
  self._send_command({
    "type": "LogMessage",
    "level": level,
    "message": message
  })

func log_error(function: String, file: String, line: int, code: String, rationale: String, error_type: int) -> void:
  self._send_command({
    "type": "LogError",
    "function": function,
    "file": file,
    "line": line,
    "code": code,
    "rationale": rationale,
    "error_type": error_type
  })

# Dynamically loaded, because older Godot versions don't have `Logger`, and so script parsing would fail.
const _LOGGER := """
extends Logger

func _log_message(message: String, error: bool) -> void:
  GDPatch.log_message("error" if error else "info", message)

func _log_error(function: String, file: String, line: int, code: String, rationale: String, _editor_notify: bool,
    error_type: ErrorType, _script_backtraces: Array[ScriptBacktrace]) -> void:
  GDPatch.log_error(function, file, line, code, rationale, error_type)

func register() -> void:
  OS.add_logger(self)
"""
