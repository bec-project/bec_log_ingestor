use std::io::Result;
fn main() -> Result<()> {
    prost_build::compile_protos(&["src/prometheus_write.proto"], &["src/"])?;
    Ok(())
}
