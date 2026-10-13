use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(proto_dir_arg) = std::env::args().nth(1) else {
        eprintln!("Usage: generate-proto <proto-dir>");
        std::process::exit(1);
    };

    let proto_dir = PathBuf::from(proto_dir_arg);
    let proto_file = proto_dir.join("codex.thread_config.v1.proto");

    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .out_dir(&proto_dir)
        .compile_protos(&[proto_file], &[proto_dir])?;

    // Preserve the repository's annotation for prost's generated enum methods.
    let generated = proto_dir.join("codex.thread_config.v1.rs");
    let content = std::fs::read_to_string(&generated)?;
    std::fs::write(
        generated,
        format!("#![allow(clippy::trivially_copy_pass_by_ref)]\n\n{content}"),
    )?;

    Ok(())
}
