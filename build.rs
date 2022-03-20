use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let window_title = env::var("WINDOW_TITLE").unwrap_or("GoodEnough Atari Emulator".to_string());
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let embed_binaries = env::var("EMBED_BINARIES")
        .map(|s| {
            s.split(",")
                .map(|s| {
                    s.to_string()
                        .splitn(2, "=")
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .ok();
    let dest_path = Path::new(&out_dir).join("build_config.rs");

    let mut contents = String::new();

    contents.push_str(&format!("const WINDOW_TITLE: &str = {:?};\n", window_title));

    match embed_binaries {
        Some(binaries) => {
            contents.push_str("pub fn embed_binaries(system: &mut AtariSystem, cpu: &mut CPU) {\n");
            for bin in binaries {
                let key = &bin[0];
                let path = &bin[1];
                let path = format!("{}/{}", manifest_dir, path);
                contents.push_str(&format!("let data = include_bytes!({:?});\n", path));
                contents.push_str(&format!(
                    "set_binary(system, cpu, {:?}, {:?},Some(data));\n",
                    key, path
                ))
            }
            contents.push_str("}\n");
        }
        None => (),
    };

    fs::write(dest_path, contents).unwrap();
    println!("cargo:rerun-if-env-changed=WINDOW_TITLE");
    println!("cargo:rerun-if-env-changed=EMBED_BINARIES");
}
