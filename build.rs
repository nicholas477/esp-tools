use embed_manifest::{embed_manifest, manifest::AssemblyIdentity, new_manifest};

fn windows_build() {
    let manifest = new_manifest("YourAppName").dependency(
        // In 1.5.0, Component was replaced by AssemblyIdentity
        // Parameters: name, version as [u16; 4], and public_key_token as u64
        AssemblyIdentity::new(
            "Microsoft.Windows.Common-Controls",
            [6, 0, 0, 0],
            0x6595_b641_44cc_f1df,
        ), // Default architecture is "*", but you can chain it explicitly if needed
    );

    embed_manifest(manifest).expect("Failed to embed application manifest");
}

fn main() {
    // Only apply the manifest logic when compiling for Windows
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        windows_build();
    }

    println!("cargo:rerun-if-changed=build.rs");
}
