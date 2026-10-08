// ReSharper disable InconsistentNaming

namespace GDPatchSharp;

public interface IGDPatch {
    List<ModInfo> GetMods();
    string GetRootDirectory();
    string? GetModDirectory(string modId);
    object? GetConfigOption(string modId, string section, string option);
    void SetConfigOption(string modId, string section, string option, object? value);
}
