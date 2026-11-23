@tool
class_name GDPatchExportPlatform
extends EditorExportPlatformExtension

func _get_name():
	return "GDPatch Mod"

const GDPATCH_LOGO := preload("res://addons/gdpatch-editor/logo.png")
func _get_logo() -> Texture2D:
	var image = GDPATCH_LOGO.get_image()
	var size = EditorInterface.get_editor_scale() * 32
	image.resize(size, size)
	return ImageTexture.create_from_image(image)
	
func _get_os_name() -> String:
	return "hello!!!"

func _get_platform_features() -> PackedStringArray:
	return PackedStringArray()

func _get_preset_features(preset: EditorExportPreset) -> PackedStringArray:
	return PackedStringArray()

func _get_binary_extensions(preset: EditorExportPreset) -> PackedStringArray:
	# TODO: we should export all of them
	return PackedStringArray()

func _has_valid_export_configuration(preset: EditorExportPreset, debug: bool) -> bool:
	if preset.get_encrypt_pck():
		set_config_error("Mod PCKs cannot be encrypted")
		return false
		
	# warnings below this point
	if preset.get_patches():
		set_config_error("Mod PCKs shouldn't be exported as patches")
		return true

	return true

func _has_valid_project_configuration(preset: EditorExportPreset) -> bool:	
	return true
	
func _export_project(preset: EditorExportPreset, debug: bool, path: String, flags: int) -> Error:
	print(EditorExportPlatform.get_forced_export_files(preset))
	print(get_internal_export_files(preset, debug))
	
	print("ex port erd")
	return Error.OK
