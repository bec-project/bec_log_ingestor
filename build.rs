use std::env::consts;
use std::io::Result;
use std::process::Command;

use which::which;

fn main() -> Result<()> {
    println!("Build setup for bec_log_ingestor.");
    print!("Assuring availability of protoc... ");
    let protoc_exists = which("protoc");
    if protoc_exists.is_err() {
        println!("protoc not found.");
        if consts::OS != "linux" {
            println!("Auto-fetching of protoc is only intended for use in CI, install it manually.")
        } else {
            let res = Command::new("sudo").arg("apt-get").arg("update").output();
            dbg!(&res);
            if res.is_err() {};
            let res = Command::new("sudo")
                .arg("apt-get")
                .arg("install")
                .arg("protobuf-compiler")
                .output();
            dbg!(&res);
            if res.is_err() {};
        }
    };

    prost_build::compile_protos(&["src/prometheus_write.proto"], &["src/"])?;
    Ok(())
}
