fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/wallet.proto");
    println!("cargo:rerun-if-changed=proto/swap_client.proto");
    tonic_prost_build::configure().compile_protos(&["proto/wallet.proto", "proto/swap_client.proto"], &["proto"])?;
    Ok(())
}
