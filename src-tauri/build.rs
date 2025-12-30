fn main() {
    // Add framework search path for Syphon on macOS
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-search=framework=/Library/Frameworks");
    }

    tauri_build::build()
}
