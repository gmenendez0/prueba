use std::net::TcpListener;
use std::process::exit;

pub(crate) fn get_tcp_listener_or_kill_process(port: u32) -> TcpListener {
    match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Error al abrir el puerto {}: {}", port, e);
            exit(1)
        }
    }
}