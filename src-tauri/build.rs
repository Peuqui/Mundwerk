fn main() {
    // The program icon (icons/icon.ico) is embedded in the exe as a Windows resource by the build script below. Cargo does not
    // run it again for a changed icon on its own, so an icon made anew stayed the old one in the exe (the microphone).
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build();
}
