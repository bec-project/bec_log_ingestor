use std::env::consts;
use std::io::Result;
use std::process::Command;

use which::which;

fn main() -> Result<()> {
    let protoc_exists = which("protoc");
    if protoc_exists.is_err() {
        if consts::OS != "linux" {
            println!("Auto-fetching of protoc is only intended for use in CI, install it manually.")
        } else {
            _ = Command::new("sudo").arg("apt-get").arg("update").output();
            _ = Command::new("sudo")
                .arg("apt-get")
                .arg("install")
                .arg("protobuf-compiler")
                .output();
        }
    };
    prost_build::compile_protos(&["src/prometheus_write.proto"], &["src/"])?;
    Ok(())
}
