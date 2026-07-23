extends Node

const MOD_ROOT: String = "res://gdpatch/mods/"
var mod_instances = {}
var mutex: Mutex
var file: FileAccess
var seq = 0
var mods: Array = []

func _init() -> void:
  self.mutex = Mutex.new()
  self.file = FileAccess.open("gdpatch-ipc", FileAccess.READ_WRITE)

  var resp = self._send_command_with_response({
    "type": "GetModList"
  })
  self.mods.assign(resp.value)

  for mod in self.mods:
    self._load_mod(mod["id"])

func _process(_delta: float) -> void:
  var data = self._read_response()
  if data != null: print(data)

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
  self.file.store_line(str)
  self.mutex.unlock()

func _read_response():
  self.mutex.lock()
  var str = self.file.get_line()
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

  # printerr("No files available for mod " + mod_id + "!")
  return

func get_mod(mod_id: String) -> Node:
  return self.mod_instances[mod_id]

func get_mods() -> Array:
  return self.mods

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
