namespace GDPatchSharp.Loader;

internal class GDPatch : IGDPatch, IDisposable {
    internal readonly GDPatchIpc Ipc;
    private readonly List<ModInfo> mods;
    private readonly string rootDirectory;

    public GDPatch() {
        this.Ipc = new GDPatchIpc();

        this.mods = this.Ipc
                        .SendCommandWithResponse<GDPatchIpc.ModListIpcResponse>(
                             new GDPatchIpc.GetModListIpcCommand())
                        .Value;
        this.rootDirectory = this
                            .Ipc.SendCommandWithResponse<GDPatchIpc.RootDirectoryIpcResponse>(
                                 new GDPatchIpc.GetRootDirectoryIpcCommand())
                            .Value;
    }

    public List<ModInfo> GetMods() => new(this.mods);

    public string GetRootDirectory() => this.rootDirectory;

    public string? GetModDirectory(string modId)
        => this.Ipc.SendCommandWithResponse<GDPatchIpc.ModDirectoryIpcResponse>(
            new GDPatchIpc.GetModDirectoryIpcCommand {
                ModId = modId
            }).Value;

    // TODO
    public object? GetConfigOption(string modId, string section, string option) => throw new NotImplementedException();

    public void SetConfigOption(string modId, string section, string option, object? value) {
        throw new NotImplementedException();
    }

    public void Dispose() {
        this.Ipc.Dispose();
    }
}
