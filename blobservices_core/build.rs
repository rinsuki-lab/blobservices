fn main() -> std::io::Result<()> {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let descriptor_path = out_dir.join("proto_descriptor.bin");

    let files = [
        "proto/manager.proto",
        "proto/storage.proto",
        "proto/transform.proto",
    ];

    for file in files {
        println!("cargo:rerun-if-changed={}", file);
    }

    let mut config = prost_build::Config::new();
    config
        // Save descriptors to file
        .file_descriptor_set_path(&descriptor_path)
        // Override prost-types with pbjson-types
        .compile_well_known_types()
        .extern_path(".google.protobuf", "::pbjson_types");

    let descriptors = config.load_fds(&files, &["proto/"])?;
    let transform_parameters = descriptors
        .file
        .iter()
        .find(|file| file.package() == "blobservices.transform")
        .and_then(|file| {
            file.message_type
                .iter()
                .find(|message| message.name() == "TransformParameters")
        })
        .expect("TransformParameters must exist");
    generate_transform_parameters(
        &out_dir,
        transform_parameters
            .field
            .iter()
            .filter(|field| field.oneof_index.is_some())
            .map(|field| {
                (
                    field.name(),
                    field.number.expect("transform field must have a number"),
                )
            }),
    )?;
    config.compile_fds(descriptors)?;

    let descriptor_set = std::fs::read(descriptor_path)?;
    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_set)?
        .emit_fields()
        .build(&[".blobservices"])?;
    Ok(())
}

fn generate_transform_parameters<'a>(
    out_dir: &std::path::Path,
    fields: impl IntoIterator<Item = (&'a str, i32)>,
) -> std::io::Result<()> {
    let fields: Vec<_> = fields
        .into_iter()
        .map(|(name, number)| {
            let variant: String = name
                .split('_')
                .map(|part| {
                    let mut chars = part.chars();
                    chars.next().unwrap().to_uppercase().collect::<String>() + chars.as_str()
                })
                .collect();
            (variant, number)
        })
        .collect();
    let mut tags = String::from(
        "// Generated from TransformParameters field numbers.\n\
         #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ::prost::Enumeration)]\n\
         #[repr(i32)]\n\
         pub enum TransformParametersTag {\n",
    );
    for (variant, number) in &fields {
        tags.push_str(&format!("    {variant} = {number},\n"));
    }
    tags.push_str("}\n");
    tags.push_str(
        "impl TryFrom<transform_parameters::Transform> for (TransformParametersTag, ::serde_json::Value) {\n\
             type Error = ::serde_json::Error;\n\
             fn try_from(value: transform_parameters::Transform) -> Result<Self, Self::Error> {\n\
                 match value {\n",
    );
    for (variant, _) in &fields {
        tags.push_str(&format!(
            "transform_parameters::Transform::{variant}(params) => Ok((TransformParametersTag::{variant}, ::serde_json::to_value(params)?)),\n"
        ));
    }
    tags.push_str("} } }\n");
    tags.push_str(
        "impl TryFrom<(TransformParametersTag, ::serde_json::Value)> for transform_parameters::Transform {\n\
             type Error = ::serde_json::Error;\n\
             fn try_from((tag, value): (TransformParametersTag, ::serde_json::Value)) -> Result<Self, Self::Error> {\n\
                 match tag {\n",
    );
    for (variant, _) in &fields {
        tags.push_str(&format!(
            "TransformParametersTag::{variant} => Ok(Self::{variant}(::serde_json::from_value(value)?)),\n"
        ));
    }
    tags.push_str("} } }\n");
    tags.push_str(
        r#"
impl TryFrom<TransformParameters> for (TransformParametersTag, ::serde_json::Value) {
    type Error = ::serde_json::Error;

    fn try_from(value: TransformParameters) -> Result<Self, Self::Error> {
        let transform = value.transform.ok_or_else(|| {
            <::serde_json::Error as ::serde::ser::Error>::custom("transform parameters are missing")
        })?;
        transform.try_into()
    }
}

impl TryFrom<(TransformParametersTag, ::serde_json::Value)> for TransformParameters {
    type Error = ::serde_json::Error;

    fn try_from(value: (TransformParametersTag, ::serde_json::Value)) -> Result<Self, Self::Error> {
        Ok(Self { transform: Some(value.try_into()?) })
    }
}
"#,
    );
    std::fs::write(out_dir.join("blobservices.transform.tags.rs"), tags)?;
    Ok(())
}
