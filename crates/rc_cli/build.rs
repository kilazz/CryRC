// Copyright 2001-2026 Crytek GmbH / Crytek Group. All rights reserved.
// CryEngine Resource Compiler Build Script with Slint UI and Win32 Resources

fn main() {
    // 1. Compile Slint Declarative UI Definition
    slint_build::compile("ui/crytif_dialog.slint").expect("Failed to compile Slint UI template");

    // 2. Compile Win32 PE Version Resources on Windows
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("windows")
    {
        let mut res = winres::WindowsResource::new();
        res.set_resource_file("rc.rc");
        if let Err(e) = res.compile() {
            eprintln!(
                "Warning: failed to compile Win32 resource file (rc.rc): {}",
                e
            );
        }
    }
}
