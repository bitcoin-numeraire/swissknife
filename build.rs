fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_server(false).compile(
        &["src/infra/lightning/cln/proto/node.proto"],
        &["src/infra/lightning/cln/proto"],
    )?;
    println!(
        "cargo:info=Generated CLN files from proto at {:?}",
        std::env::var("OUT_DIR").unwrap()
    );

    Ok(())
}
