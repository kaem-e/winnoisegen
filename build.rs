use winresource;

fn main() {
	if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
		let mut res = winresource::WindowsResource::new();

		res.set_icon(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.ico"));
		res.set("FileDescription", env!("CARGO_PKG_DESCRIPTION"));
		res.set("ProductName", env!("CARGO_PKG_NAME"));
		res.set("ProductVersion", env!("CARGO_PKG_VERSION"));

		res.compile().unwrap();
	}
}
