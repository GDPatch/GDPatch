namespace GDPatchSharp;

public record ModInfo {
    /// A unique ID for this mod. Mod IDs should use snake case (all lowercase, with spaces replaced with underscores).
    /// This is the only required field in the mod info.
    public string Id { get; set; } = null!;

    /// The filename of the assembly in the `dotnet` directory, if this mod includes a .NET assembly.
    /// This path is relative to the `dotnet` directory in the mod, so `ExampleMod.dll` resolves to `(mod folder)/dotnet/ExampleMod.dll`.
    public string? DotnetAssembly { get; set; }

    /// Human-readable metadata about this mod.
    public ModMeta? Meta { get; set; }

    /// Optional metadata for the mod's config options. Config options are referenced by a section ID and option ID.
    /// The "meta" option ID is reserved to represent metadata about the section itself.
    public Dictionary<string, Dictionary<string, ModConfigOptionMeta>>? Config { get; set; }
}

public record ModMeta {
    /// Pretty name for this mod.
    public string? Name { get; set; }

    /// Version number. No strict format is imposed on this, but it should be obvious to users.
    public string? Version { get; set; }

    /// A list of mod authors.
    public List<string> Authors { get; set; } = [];

    /// A short description of what this mod does.
    public string? Description { get; set; }

    /// A link to a website (such as mod page or source code) for this mod.
    public string? Website { get; set; }
}

public record ModConfigOptionMeta {
    /// Pretty name for this option.
    public string? Name { get; set; }

    /// Description for this option.
    public string? Description { get; set; }

    /// Type for this option, to be used as a hint for custom config editors.
    /// GDPatch does not perform any type checking, and the value saved in this option may not match the type.
    public ModConfigOptionType Type { get; set; }

    // /// The default value for this option.
    // /// The default value will not be written to the config file directly, but will be returned by the config APIs and displayed in comments.
    //public object? Default { get; set; }

    /// Whether to hide all comments for this option.
    public bool? Hidden { get; set; }
}

public enum ModConfigOptionType {
    String,
    Number,
    Boolean,
    Array,
    Table,
}
