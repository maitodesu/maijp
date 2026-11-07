// Windows: embed the app icon in maijp.exe (Explorer, taskbar, Start menu).
fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon/maijp.ico");
        res.compile().expect("embed the Windows icon");
    }
}
