fn main() {
    #[cfg(windows)]
    {
        // Ensure Windows SDK bin directory is in PATH for rc.exe
        let sdk_bin = r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64";
        if std::path::Path::new(sdk_bin).exists() {
            if let Ok(path) = std::env::var("PATH") {
                std::env::set_var("PATH", format!("{};{}", sdk_bin, path));
            }
        }

        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app.ico");
        res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
        </requestedPrivileges>
    </security>
</trustInfo>
<application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
        <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
        <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2, PerMonitor</dpiAwareness>
    </windowsSettings>
</application>
</assembly>
"#);
        if let Err(e) = res.compile() {
            panic!("Failed to compile windows resources: {}", e);
        }
    }
}
