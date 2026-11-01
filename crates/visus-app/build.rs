fn main() {
    // Embeds the app icon and version info into Visus.exe (taskbar, Explorer, installer).
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/visus.ico");
        res.set("ProductName", "Visus");
        res.set("FileDescription", "Visus");
        res.set("CompanyName", "DevDad");
        res.compile().expect("embed Windows resources");
    }
    println!("cargo:rerun-if-changed=assets/visus.ico");
}
