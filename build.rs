
use std::{env};
use std::fs;
use std::io::{Read};
use std::path::Path;
use glsl_to_spirv::ShaderType;
use glsl_to_spirv as glsl;
use std::error::Error;

fn main() {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let src_dir = Path::new(&manifest_dir).join("src").join("shaders");
    let dst_dir = Path::new(&out_dir).join("spirv");
    if dst_dir.exists() {
        // TODO: Clean?
    } else {
        fs::create_dir_all(&dst_dir).unwrap();
    }
    for entry in src_dir.read_dir().unwrap() {
        if let Ok(src_path) = entry.map(|e| e.path()) {
            if !src_path.is_file() { continue }
            let src_file = src_path.to_str().unwrap();
            let shader_type = get_shader_type(&src_path)
                .expect(&format!("Cannot compile shader {}", src_file));

            let src = fs::read_to_string(&src_path).unwrap();
            let dst = dst_dir.join(src_path.with_extension("spv").file_name().unwrap());
            let dst_file = dst.to_str().unwrap();
            println!("cargo:warning=Compiling shader {} -> {}", src_file, dst_file);
            compile_shader(&src, shader_type, &dst).unwrap();
            println!("cargo:rerun-if-changed={}", src_file);

        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}

fn get_shader_type(path: &Path) -> Result<ShaderType, String> {

    match path
        .file_name().ok_or(format!("Invalid path"))?
        .to_str().ok_or(format!("Invalid path"))?
        .rsplitn(3, '.')
        .take(2)
        .collect::<Vec<&str>>()[..]
    {
        ["glsl", t] => match t {
            "frag" => Ok(ShaderType::Fragment),
            "vert" => Ok(ShaderType::Vertex),
            "comp" => Ok(ShaderType::Compute),
            x => Err(format!("Unknown shader type {:?}", x))
        },
        [e, _] => Err(format!("Invalid extension: {:?}", e)),
        _ => Err(format!("File name does not contain shader type"))
    }
}

fn compile_shader<P: AsRef<Path>>(src: &str, shader_type: glsl::ShaderType, output_path: P) -> Result<(), Box<dyn Error>> {
    let mut compiled = glsl::compile(src, shader_type)?; //.map_err(|e| => );
    let mut bytes = Vec::new();
    compiled.read_to_end(&mut bytes)?;
    fs::write(output_path,bytes)?;
    Ok(())
}