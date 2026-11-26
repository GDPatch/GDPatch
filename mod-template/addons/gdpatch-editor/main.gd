@tool
extends EditorPlugin

var _export_platform := GDPatchExportPlatform.new()
var _export_plugin := GDPatchExportPlugin.new()

func _enter_tree():
	print("woof")

	add_export_platform(_export_platform)
	add_export_plugin(_export_plugin)

	var project_path := ProjectSettings.globalize_path("res://")
	var data = JSON.parse_string(FileAccess.get_file_as_string("res://gdpatch_editor.json"))
	var pack_path = data["pack_path"]
	var pack = GDPatchPack.new()
	pack.init(pack_path)
	print("set pack!")

	Filesilly.init(project_path, pack)
	print("setup filesilly!")

func _exit_tree():
	print("bye")

	remove_export_platform(_export_platform)
	remove_export_plugin(_export_plugin)
