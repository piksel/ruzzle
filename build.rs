
use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;
use glsl_to_spirv::ShaderType;
use glsl_to_spirv as glsl;

fn main() {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();

    let src_dir = Path::new(&manifest_dir).join("src/shaders");
    for entry in src_dir.read_dir().unwrap() {
        if let Ok(entry) = entry {
            let file = entry.file_name();
            let shader_type = match file.to_str().unwrap().split('.').skip(1).take(1).collect() {
                "frag" => ShaderType::Fragment,
                "vert" => ShaderType::Vertex,
                "comp" => ShaderType::Compute,
                x => {
                    println!("Unknown shader type {:?} for {:?}", x, file); continue
                }
            };

            let src = fs::read_to_string(entry).unwrap();
            compile_shader(&src, shader_type, entry.path().set_extension("spv"));
            println!("{:?}", file);
        }
    }

    let bg_frag = include_str!("src/shaders/background.frag.glsl");
    let bg_frag_out = Path::new(&manifest_dir).join("shaders/background.frag.spv");

    compile_shader(include_str!("src/shaders/background.frag.glsl"),
                   ShaderType::Fragment,
                   Path::new(&manifest_dir).join("shaders/background.frag.spv"));


    panic!();
    // let vs = include_str!("src/shader.vert");
    // let mut vs_compiled = glsl_to_spirv::compile(vs, glsl_to_spirv::ShaderType::Vertex).unwrap();
    // let mut vs_bytes = Vec::new();
    // vs_compiled.read_to_end(&mut vs_bytes).unwrap();
    // fs::write(
    //     &Path::new(&manifest_dir).join("src/shader.vert.spv"),
    //     vs_bytes,
    // )
    //     .unwrap();
    //
    // let fs = include_str!("src/shader.frag");
    // let mut fs_compiled = glsl_to_spirv::compile(fs, glsl_to_spirv::ShaderType::Fragment).unwrap();
    // let mut fs_bytes = Vec::new();
    // fs_compiled.read_to_end(&mut fs_bytes).unwrap();
    // fs::write(
    //     &Path::new(&manifest_dir).join("src/shader.frag.spv"),
    //     fs_bytes,
    // )
    //     .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}

fn compile_shader<P: AsRef<Path>>(src: &str, shader_type: glsl::ShaderType, output_path: P) {
    let mut compiled = glsl::compile(src, shader_type).unwrap();
    let mut bytes = Vec::new();
    compiled.read_to_end(&mut bytes).unwrap();
    fs::write(
        output_path,
        bytes,
    )
    .unwrap();
}