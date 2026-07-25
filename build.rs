fn main() {
	if let Ok(path) = std::env::var("CARGO_MANIFEST_DIR") {
		let env_path = std::path::Path::new(&path).join(".env");
		if env_path.exists() {
			if let Ok(content) = std::fs::read_to_string(&env_path) {
				for line in content.lines() {
					let line = line.trim();
					if line.is_empty() || line.starts_with('#') {
						continue;
					}
					if let Some((key, value)) = line.split_once('=') {
						println!("cargo:rustc-env={}={}", key.trim(), value.trim());
					}
				}
			}
		}
	}
	println!("cargo:rerun-if-changed=.env");
}
