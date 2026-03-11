/// Lapis Sapientiae — Core Agent entry point.

#[tokio::main]
async fn main() {
    println!("===========================================");
    println!("  Lapis Sapientiae — Core Agent v0.1.0");
    println!("===========================================");

    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = logging::init() {
        eprintln!("Failed to init logging: {e}");
        std::process::exit(1);
    }

    let mode = if cfg.simulation_mode {
        "SIMULATION"
    } else {
        "REAL"
    };
    println!("  Mode: {mode}");
    println!("  IPC:  {}:{}", cfg.ipc_transport, cfg.ipc_port);
    println!("===========================================");
    println!();

    let server = match ipc::IpcServer::new(cfg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create IPC server: {e}");
            std::process::exit(1);
        }
    };

    println!("Core agent starting IPC server...");
    if let Err(e) = server.run().await {
        eprintln!("IPC server error: {e}");
        std::process::exit(1);
    }
}
