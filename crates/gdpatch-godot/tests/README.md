# `gdpatch-godot` integration tests

Harness and custom Godot engine module for comparing the results of `gdpatch-godot` to the upstream implementation.
## Usage

Check out a copy of the Godot sources for your engine version and compile a custom build. If you want to test multiple
Godot versions at once, using `git-worktree` is recommended.

```shell
scons platform=<platform> custom_modules=<module_path> target=template_debug extra_suffix=gdpatch_test minizip=no brotli=no xaudio2=no vulkan=no opengl3=no d3d12=no x11=no wayland=no metal=no use_volk=no accesskit=no sdl=no disable_3d=yes disable_advanced_gui=yes disable_physics_2d=yes disable_physics_3d=yes disable_navigation_2d=yes disable_navigation_3d=yes disable_xr=yes disable_overrides=yes disable_path_overrides=no modules_enabled_by_default=no module_gdscript_enabled=yes module_gdpatch_test_enabled=yes
```

Setting a `cache_path` in the scons build arguments helps with build times when building many versions of the engine.
Using a faster linker (such as [mold](https://github.com/rui314/mold)) helps with build times - set `linker=mold`.

Set the `GDPATCH_TEST_GODOT` environment variable to the path to your newly built Godot template.
Make sure you point to the console binary (ending `.console.exe`) if you are on Windows. Then you can run the test suite
with `cargo test --package gdpatch-godot --test difftest`. It's recommended to run with 
[`cargo-llvm-cov`](https://crates.io/crates/cargo-llvm-cov) to get coverage reports.

You can set the `GDPATCH_TEST_CORPUS` environment variable to a different location to run tests on a different corpus. 
The directory is walked recursively, so setting it to a Godot project root will attempt to parse everything in the 
project.