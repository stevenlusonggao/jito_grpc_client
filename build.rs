fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe { std::env::set_var("PROTOC", protobuf_src::protoc()) };
    tonic_prost_build::configure()
        .build_server(false)
        .compile_protos(
            &[
                "proto/searcher.proto",
                "proto/bundle.proto",
                "proto/packet.proto",
                "proto/shared.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}
