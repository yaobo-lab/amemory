//! Build script to embed Windows resources (Application icon and metadata).

fn main() {
    println!("cargo:rerun-if-changed=assets/LOGO_notext.ico");
    println!("cargo:rerun-if-changed=assets/LOGO_notext.png");

    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/LOGO_notext.ico");
        res.set("FileDescription", "Amemory");
        res.set("ProductName", "Amemory");
        res.set("OriginalFilename", "xcat.exe");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {e}");
        }
    }
}
