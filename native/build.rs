fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut resource = winres::WindowsResource::new();
        resource
            .set_icon("icons/icon.ico")
            .set("ProductName", "订阅镜")
            .set("FileDescription", "订阅镜 · ChatGPT 订阅查询")
            .set("CompanyName", "PascalePaF")
            .set("LegalCopyright", "Copyright © 2026 PascalePaF")
            .set("OriginalFilename", "SubscriptionLens.exe")
            .set("ProductVersion", env!("CARGO_PKG_VERSION"))
            .set("FileVersion", env!("CARGO_PKG_VERSION"));
        resource
            .compile()
            .expect("failed to embed Windows resources");
    }
}
