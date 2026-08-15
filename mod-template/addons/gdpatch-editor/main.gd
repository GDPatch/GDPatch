@tool
extends EditorPlugin

func _enter_tree():
	var project_path = ProjectSettings.globalize_path("res://")
	var data = JSON.parse_string(FileAccess.get_file_as_string("res://gdpatch_editor.json"))
	var pack_path = data["pack_path"]
	GDPatchPack.set_pack(project_path, pack_path)

func _exit_tree():
	print("bye")
