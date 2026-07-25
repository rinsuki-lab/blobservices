use std::{fs::File, io::Read};

pub fn load_from_env_or_file_or_panic(env_base: &str) -> String {
    let config = std::env::var(env_base);
    match config {
        Ok(s) => return s,
        Err(std::env::VarError::NotPresent) => (),
        _ => _ = config.expect(&format!("{} env value's contents are not valid", env_base)),
    };

    let config_file_env = format!("{}_FILE", env_base);
    let config_file = std::env::var_os(&config_file_env);
    let config_file = match config_file {
        Some(p) => p,
        None => {
            panic!("{} or {} env is required", env_base, config_file_env)
        }
    };

    let mut config = String::new();

    File::options()
        .read(true)
        .open(&config_file)
        .unwrap_or_else(|e| {
            panic!(
                "Failed to open {} (from {} env) due to: {:?}",
                config_file.to_string_lossy(),
                config_file_env,
                e
            )
        })
        .read_to_string(&mut config)
        .unwrap_or_else(|e| {
            panic!(
                "Failed to read {} (from {} env) due to: {:?}",
                config_file.to_string_lossy(),
                config_file_env,
                e
            );
        });

    config
}
