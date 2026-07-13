use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=java/RTMLShim.java");
    println!("cargo:rerun-if-changed=assets/rtml.rc");
    println!("cargo:rerun-if-changed=assets/icon.ico");

    let out_dir = env::var("OUT_DIR").unwrap();
    let out = Path::new(&out_dir);
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let target = env::var("TARGET").unwrap_or_default();

    // compile RTMLShim.java into a jar that gets embedded via include_bytes!
    let status = Command::new("javac")
        .arg("-source")
        .arg("8")
        .arg("-target")
        .arg("8")
        .arg("-d")
        .arg(out.to_str().unwrap())
        .arg("java/RTMLShim.java")
        .status()
        .expect("Failed to run javac - is a JDK installed?");

    assert!(status.success(), "javac failed to compile RTMLShim.java");

    // package into a jar
    let jar_path = out.join("rtml-shim.jar");
    let status = Command::new("jar")
        .arg("cfe")
        .arg(jar_path.to_str().unwrap())
        .arg("RTMLShim")
        .arg("-C")
        .arg(out.to_str().unwrap())
        .arg("RTMLShim.class")
        .status()
        .expect("Failed to run jar - is a JDK installed?");

    assert!(status.success(), "jar failed to create rtml-shim.jar");

    // embed Windows icon resource via windres -> COFF object -> linker
    if target.contains("windows") {
        let rc_path = Path::new(&manifest_dir).join("assets").join("rtml.rc");
        let obj_path = out.join("rtml_icon.o");

        let windres = if Command::new("x86_64-w64-mingw32-windres")
            .arg("--version")
            .output()
            .is_ok()
        {
            "x86_64-w64-mingw32-windres"
        } else if Command::new("windres")
            .arg("--version")
            .output()
            .is_ok()
        {
            "windres"
        } else {
            eprintln!("cargo:warning=windres not found, skipping icon embedding");
            return;
        };

        let status = Command::new(windres)
            .arg("-O")
            .arg("coff")
            .arg(&rc_path)
            .arg("-o")
            .arg(&obj_path)
            .current_dir(Path::new(&manifest_dir).join("assets"))
            .status()
            .expect("Failed to run windres");

        if !status.success() {
            eprintln!("cargo:warning=windres failed: {}", status);
            return;
        }

        println!("cargo:rustc-link-arg={}", obj_path.to_str().unwrap());
    }
}
