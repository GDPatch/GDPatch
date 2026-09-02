using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace GDPatchSharp.Loader;

internal class GDPatchIpc : IDisposable {
    private readonly FileStream file;
    private readonly object @lock = new();
    private int seq = 0;

    private static readonly JsonSerializerOptions SerializerOptions = new() {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        Converters = { new JsonStringEnumConverter<ModConfigOptionType>() }
    };

    public GDPatchIpc() {
        this.file = File.Open("gdpatch-ipc", FileMode.Open, FileAccess.ReadWrite);
    }

    public T SendCommandWithResponse<T>(IpcCommand command) where T : IpcResponse {
        var data = JsonSerializer.SerializeToNode(command, SerializerOptions);
        if (data is null) throw new Exception("Failed to serialize command");

        int thisSeq;
        lock (this.@lock) thisSeq = this.seq++;
        data["seq"] = thisSeq;

        this.SendCommand(JsonSerializer.Serialize(data, SerializerOptions));

        // FIXME: broken filesilly write will make games spin forever with this loop
        while (true) {
            // TODO: cache seq if we receive out of order somehow?
            var resp = this.ReadResponse();
            if (
                resp is not null
                && resp["seq"]?.AsValue()?.TryGetValue(out int respSeq) == true
                && respSeq == thisSeq
            ) {
                // Remove the extra key so the serializer is happy
                var obj = resp.AsObject();
                obj.Remove("seq");

                var respObj = obj.Deserialize<IpcResponse>(SerializerOptions);
                return respObj as T ?? throw new Exception("Failed to deserialize response");
            }
        }
    }

    public void SendCommand(IpcCommand command) {
        this.SendCommand(JsonSerializer.Serialize(command, SerializerOptions));
    }

    private void SendCommand(string str) {
        var data = Encoding.UTF8.GetBytes(str.Trim() + "\n");
        lock (this.@lock) {
            this.file.Write(data);
            this.file.Flush();
        }
    }

    private JsonNode? ReadResponse() {
        lock (this.@lock) {
            using var reader = new StreamReader(this.file, leaveOpen: true);
            var line = reader.ReadLine();
            return line is null ? null : JsonSerializer.Deserialize<JsonNode>(line, SerializerOptions);
        }
    }

    public void Dispose() {
        this.file.Dispose();
    }

    [JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]
    [JsonDerivedType(typeof(GetModListIpcCommand), typeDiscriminator: "GetModList")]
    [JsonDerivedType(typeof(GetRootDirectoryIpcCommand), typeDiscriminator: "GetRootDirectory")]
    [JsonDerivedType(typeof(GetModDirectoryIpcCommand), typeDiscriminator: "GetModDirectory")]
    public record IpcCommand;

    public record GetModListIpcCommand : IpcCommand;
    public record GetRootDirectoryIpcCommand : IpcCommand;

    public record GetModDirectoryIpcCommand : IpcCommand {
        public string ModId { get; set; } = null!;
    }

    [JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]
    [JsonDerivedType(typeof(ModListIpcResponse), typeDiscriminator: "ModList")]
    [JsonDerivedType(typeof(RootDirectoryIpcResponse), typeDiscriminator: "RootDirectory")]
    [JsonDerivedType(typeof(ModDirectoryIpcResponse), typeDiscriminator: "ModDirectory")]
    public record IpcResponse;

    public record ModListIpcResponse : IpcResponse {
        public List<ModInfo> Value { get; set; } = [];
    }

    public record RootDirectoryIpcResponse : IpcResponse {
        public string Value { get; set; } = null!;
    }

    public record ModDirectoryIpcResponse : IpcResponse {
        public string? Value { get; set; }
    }
}
