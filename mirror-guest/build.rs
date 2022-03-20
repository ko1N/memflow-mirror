extern crate winres;

fn main() {
    // compile with default values from Cargo.toml
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/icon.ico");
    res.set_manifest(
        r#"
    <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
    <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
        <security>
            <requestedPrivileges>
                <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
            </requestedPrivileges>
        </security>
    </trustInfo>
    </assembly>
    "#,
    );
    res.compile().ok(); // this is allowed to fail in cross-compilation scenarios
}
