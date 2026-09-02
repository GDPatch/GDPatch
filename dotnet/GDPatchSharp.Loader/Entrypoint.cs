namespace GDPatchSharp.Loader;

internal class Entrypoint {
    internal delegate void MainDelegate();

    internal static void Main() {
        try {
            var gdpatch = new GDPatch();

            foreach (var mod in gdpatch.GetMods()) {
                Console.WriteLine("mod " + mod.Id);
                if (mod.DotnetAssembly is { } assembly) {
                    var modDir = gdpatch.GetModDirectory(mod.Id);
                    if (modDir is null) continue;

                    var assemblyPath = Path.Combine(modDir, "dotnet", assembly);
                    assemblyPath = Path.GetFullPath(assemblyPath);
                    if (!File.Exists(assemblyPath)) {
                        Console.WriteLine($"Assembly for mod {mod.Id} does not exist: " + assemblyPath);
                        continue;
                    }

                    LoadMod(gdpatch, assemblyPath);
                }
            }
        } catch (Exception e) {
            Console.WriteLine(e);
        }
    }

    private static IGDPatchMod? LoadMod(IGDPatch gdpatch, string assemblyPath) {
        var fullAssemblyPath = Path.GetFullPath(assemblyPath);
        var context = new ModLoadContext(fullAssemblyPath);
        var assembly = context.LoadFromAssemblyPath(fullAssemblyPath);
        var modType = assembly.GetTypes().FirstOrDefault(t =>
            t.GetInterfaces().FirstOrDefault(i => i.FullName == typeof(IGDPatchMod).FullName) != null);

        if (modType == null) {
            Console.WriteLine($"Assembly at {assemblyPath} does not contain a mod");
            return null;
        }

        var ctor = modType.GetConstructor([typeof(IGDPatch)]);

        return ctor is not null
            ? ctor.Invoke([gdpatch]) as IGDPatchMod
            : Activator.CreateInstance(modType) as IGDPatchMod;
    }
}
