/// Lapis Sapientiae — Core Agent entry point.
fn main() {
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
    println!("Core agent is ready. Waiting for instructions...");

    // Phase 1: no event loop yet — just demonstrate the pipeline.
    match orchestrator::handle_instruction("Phase 1 smoke test", &cfg) {
        Ok(summary) => println!("Result: {summary}"),
        Err(e) => eprintln!("Error: {e}"),
    }
}
