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
    }

    public static class EspTools
    {
        public static string ScanToJson(string espPath)
        {
            if (espPath == null)
                throw new ArgumentNullException(nameof(espPath));

            byte[] utf8Path = Encoding.UTF8.GetBytes(espPath);

            GCHandle pathHandle = GCHandle.Alloc(utf8Path, GCHandleType.Pinned);
            try
            {
                IntPtr graph = EspToolsNative.ScanEspFileGraph(
                    pathHandle.AddrOfPinnedObject(),
                    new UIntPtr((ulong)utf8Path.LongLength));

                if (graph == IntPtr.Zero)
                    return null;

                try
                {
                    IntPtr jsonBytes;
                    UIntPtr jsonLength;

                    if (!EspToolsNative.GetEspFileAssetsJson(
                        graph,
                        out jsonBytes,
                        out jsonLength))
                    {
                        return null;
                    }

                    try
                    {
                        ulong byteCount = jsonLength.ToUInt64();

                        if (byteCount > int.MaxValue)
                            throw new InvalidOperationException(
                                "The returned JSON is too large for a managed byte array.");

                        if (byteCount == 0)
                            return string.Empty;

                        byte[] json = new byte[(int)byteCount];
                        Marshal.Copy(jsonBytes, json, 0, json.Length);

                        return Encoding.UTF8.GetString(json);
                    }
                    finally
                    {
                        EspToolsNative.FreeUtf8(jsonBytes, jsonLength);
                    }
                }
                finally
                {
                    EspToolsNative.FreeEspFileGraph(graph);
                }
            }
            finally
            {
                pathHandle.Free();
            }
        }
    }
}