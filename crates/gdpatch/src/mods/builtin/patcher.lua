GDPatch.patch_project_settings(function(settings)
  table.insert(settings, 1, {
    "autoload/GDPatch",
    "*res://gdpatch/GDPatch.gd"
  })
  return settings
end)
