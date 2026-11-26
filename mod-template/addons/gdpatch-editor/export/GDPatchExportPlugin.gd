@tool
class_name GDPatchExportPlugin
extends EditorExportPlugin

func _supports_platform(platform: EditorExportPlatform) -> bool:
	return platform is GDPatchExportPlatform
	
func _get_name() -> String:
	return "GDPatch Export Plugin"

func _export_begin(features: PackedStringArray, is_debug: bool, path: String, flags: int) -> void:
	print("export start")

func _export_file(path: String, type: String, features: PackedStringArray) -> void:
	print(path)
