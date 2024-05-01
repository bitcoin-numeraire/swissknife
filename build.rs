fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("src/infra/lightning/lightningd/proto/node.proto")?;
    println!(
        "cargo:info=Generated CLN files from proto at {:?}",
        std::env::var("OUT_DIR").unwrap()
    );

    Ok(())
}
