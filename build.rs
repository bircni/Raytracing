use std::env;

fn main() -> anyhow::Result<()> {
    let target = env::var("CARGO_CFG_TARGET_OS")?;
    if target == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("./res/icon.ico");
        res.compile()?;
    }
    Ok(())
}
