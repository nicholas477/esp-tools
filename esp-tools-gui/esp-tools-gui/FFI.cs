using System;
using System.Runtime.InteropServices;
using System.Text;

namespace esp_tools_gui
{
    internal static class EspToolsNative
    {
        // This must name the Rust cdylib, not the CLI executable.
        private const string LibraryName = "esp_tools.dll";

        [DllImport(
             LibraryName,
             EntryPoint = "esp_tools_init",
             CallingConvention = CallingConvention.Cdecl,
             ExactSpelling = true)]
        internal static extern void Init();

        [DllImport(
            LibraryName,
            EntryPoint = "esp_tools_scan_esp_file_graph",
            CallingConvention = CallingConvention.Cdecl,
            ExactSpelling = true)]
        internal static extern IntPtr ScanEspFileGraph(
            IntPtr inputFile,
            UIntPtr inputFileLenBytes);

        [DllImport(
            LibraryName,
            EntryPoint = "esp_tools_free_esp_file_graph",
            CallingConvention = CallingConvention.Cdecl,
            ExactSpelling = true)]
        internal static extern void FreeEspFileGraph(IntPtr graph);

        [DllImport(
            LibraryName,
            EntryPoint = "esp_tools_get_esp_file_assets_json",
            CallingConvention = CallingConvention.Cdecl,
            ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.I1)]
        internal static extern bool GetEspFileAssetsJson(
            IntPtr graph,
            out IntPtr jsonBytes,
            out UIntPtr jsonLenBytes);

        [DllImport(
            LibraryName,
            EntryPoint = "esp_tools_free_utf8",
            CallingConvention = CallingConvention.Cdecl,
            ExactSpelling = true)]
        internal static extern void FreeUtf8(
            IntPtr value,
            UIntPtr valueLenBytes);

        [DllImport(LibraryName,
            EntryPoint = "esp_tools_package_zip",
            CallingConvention = CallingConvention.Cdecl,
            ExactSpelling = true)]
        internal static extern bool PackageZip(
            IntPtr graph,
            IntPtr inputFile,
            UIntPtr inputFileLenBytes,
            IntPtr outputFilePath,
            UIntPtr outputFilePathLenBytes);
    }

    // Wrapper around graph type
    public class EspFileGraph
    {
        internal IntPtr Handle { get; private set; }

        internal EspFileGraph(IntPtr handle)
        {
            Handle = handle;
        }

        public bool IsValid()
        {
            return Handle != IntPtr.Zero;
        }

        ~EspFileGraph()
        {
            if (Handle != IntPtr.Zero)
            {
                EspToolsNative.FreeEspFileGraph(Handle);
                Handle = IntPtr.Zero;
            }
        }
    }

    public static class EspTools
    {
        public static (string Json, EspFileGraph Graph) ScanToJson(string espPath)
        {
            if (espPath == null)
                throw new ArgumentNullException(nameof(espPath));

            byte[] utf8Path = Encoding.UTF8.GetBytes(espPath);

            GCHandle pathHandle = GCHandle.Alloc(utf8Path, GCHandleType.Pinned);
            try
            {
                EspFileGraph graph = new EspFileGraph(EspToolsNative.ScanEspFileGraph(
                    pathHandle.AddrOfPinnedObject(),
                    new UIntPtr((ulong)utf8Path.LongLength)));

                if (!graph.IsValid())
                    return ("", null);

                try
                {
                    IntPtr jsonBytes;
                    UIntPtr jsonLength;

                    if (!EspToolsNative.GetEspFileAssetsJson(
                        graph.Handle,
                        out jsonBytes,
                        out jsonLength))
                    {
                        return ("", null);
                    }

                    try
                    {
                        ulong byteCount = jsonLength.ToUInt64();

                        if (byteCount > int.MaxValue)
                            throw new InvalidOperationException(
                                "The returned JSON is too large for a managed byte array.");

                        if (byteCount == 0)
                            return ("", graph);

                        byte[] json = new byte[(int)byteCount];
                        Marshal.Copy(jsonBytes, json, 0, json.Length);

                        return (Encoding.UTF8.GetString(json), graph);
                    }
                    finally
                    {
                        EspToolsNative.FreeUtf8(jsonBytes, jsonLength);
                    }
                }
                finally
                {
                    //EspToolsNative.FreeEspFileGraph(graph.Handle);
                }
            }
            finally
            {
                pathHandle.Free();
            }
        }

        public static bool PackageZip(EspFileGraph graph, string inputFilePath, string outputFilePath)
        {
            if (graph == null || !graph.IsValid())
                throw new ArgumentNullException(nameof(graph));
            if (inputFilePath == null)
                throw new ArgumentNullException(nameof(inputFilePath));
            if (outputFilePath == null)
                throw new ArgumentNullException(nameof(outputFilePath));
            byte[] utf8InputPath = Encoding.UTF8.GetBytes(inputFilePath);
            byte[] utf8OutputPath = Encoding.UTF8.GetBytes(outputFilePath);
            GCHandle inputHandle = GCHandle.Alloc(utf8InputPath, GCHandleType.Pinned);
            GCHandle outputHandle = GCHandle.Alloc(utf8OutputPath, GCHandleType.Pinned);
            try
            {
                return EspToolsNative.PackageZip(
                    graph.Handle,
                    inputHandle.AddrOfPinnedObject(),
                    new UIntPtr((ulong)utf8InputPath.LongLength),
                    outputHandle.AddrOfPinnedObject(),
                    new UIntPtr((ulong)utf8OutputPath.LongLength));
            }
            finally
            {
                inputHandle.Free();
                outputHandle.Free();
            }
        }
    }
}