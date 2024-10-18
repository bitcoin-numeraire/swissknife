use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .build_server(false)
        .compile_protos(
            &[
                "src/infra/lightning/cln/proto/node.proto",
                "src/infra/lightning/lnd/proto/lightning.proto",
            ],
            &[
                "src/infra/lightning/cln/proto",
                "src/infra/lightning/lnd/proto",
            ],
        )?;
    println!(
        "cargo:info=Generated files from proto at {:?}",
        std::env::var("OUT_DIR").unwrap()
    );

    println!(
        "cargo:rustc-env=CARGO_PKG_VERSION={}",
        env!("CARGO_PKG_VERSION")
    );
    println!("cargo:rustc-env=BUILD_TIME={}", Utc::now().to_rfc3339());

    Ok(())
}
