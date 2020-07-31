fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .out_dir("proto/pb")
        .compile(&["proto/crobat.proto"], &["proto"])?;
    Ok(())
}
