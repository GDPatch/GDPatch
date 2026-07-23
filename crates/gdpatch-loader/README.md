# `gdpatch-loader`

Loads GDPatch into the target game process.

## Windows

Load into the process by placing next to the game executable as `winmm.dll`.

On Wine, you need to set a DLL override. To do this in Steam, go to Properties > General > Launch Options and enter `WINEDLLOVERRIDES="winmm=n,b" %command%`.

## Linux

Load into the process by using the `LD_PRELOAD` environment variable (e.g. `LD_PRELOAD=/path/to/libgdpatch_loader.so ./game`).
