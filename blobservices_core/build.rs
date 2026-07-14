fn main() -> std::io::Result<()> {
    let descriptor_path =
        std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("proto_descriptor.bin");

    let files = ["proto/manager.proto", "proto/storage.proto"];

    for file in files {
        println!("cargo:rerun-if-changed={}", file);
    }

    prost_build::Config::new()
        // Save descriptors to file
        .file_descriptor_set_path(&descriptor_path)
        // Override prost-types with pbjson-types
        .compile_well_known_types()
        .extern_path(".google.protobuf", "::pbjson_types")
        // Generate prost structs
        .compile_protos(&files, &["proto/"])?;

    let descriptor_set = std::fs::read(descriptor_path)?;
    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_set)?
        .emit_fields()
        .build(&[".blobservices"])?;
    Ok(())
}
