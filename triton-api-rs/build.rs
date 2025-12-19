fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(false)
        .out_dir("src") // you can change the generated code's location
        .compile_protos(
            &["proto/common/protobuf/grpc_service.proto"],
            &["proto/common/protobuf"], // specify the root location to search proto dependencies
        )?;
    Ok(())
}
