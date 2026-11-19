using System.Reflection;
using System.Runtime.Loader;

namespace GDPatchSharp.Loader;

internal class ModLoadContext(string path) : AssemblyLoadContext(isCollectible: true) {
    private readonly AssemblyDependencyResolver resolver = new(path);

    protected override Assembly Load(AssemblyName assemblyName) {
        var gdpatchAssembly = typeof(IGDPatch).Assembly;
        if (assemblyName.Name == "GDPatchSharp") return gdpatchAssembly;

        var existing = gdpatchAssembly.GetReferencedAssemblies().FirstOrDefault(a => a.Name == assemblyName.Name);
        if (existing is not null) return Assembly.Load(existing);

        var assemblyPath = this.resolver.ResolveAssemblyToPath(assemblyName);
        if (assemblyPath is null) return null!;

        return this.LoadFromAssemblyPath(assemblyPath);
    }

    protected override nint LoadUnmanagedDll(string unmanagedDllName) {
        var libraryPath = this.resolver.ResolveUnmanagedDllToPath(unmanagedDllName);
        return libraryPath is not null
            ? this.LoadUnmanagedDllFromPath(libraryPath)
            : 0;
    }
}
