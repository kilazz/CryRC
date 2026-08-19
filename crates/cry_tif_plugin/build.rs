fn main() {
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("windows")
    {
        let mut res = winres::WindowsResource::new();
        res.set_resource_file("CryTIFPlugin.rc");
        if let Err(e) = res.compile() {
            eprintln!("Warning: failed to compile Win32 resource: {}", e);
        }
    }
}
