use std::env;
use std::io::BufRead;
use std::path::Path;
use crate::{file_handler, ARGS_EXPECTED};
use crate::process::Process;

pub(crate) fn check_args() {
    let args: Vec<String> = env::args().collect();

    if args.len() != ARGS_EXPECTED {
        eprintln!("Error en args. Uso: cargo run -- <pid> <port> <other_processes_filename>");
        std::process::exit(1);
    }
}

pub(crate) fn get_process_id() -> u32 {
    let args: Vec<String> = env::args().collect();

    let pid: u32 = match args[1].parse() {
        Ok(id) => id,
        Err(_) => {
            eprintln!("Error: El argumento pid debe ser un número entero.");
            std::process::exit(1);
        }
    };

    pid
}

pub(crate) fn get_process_port() -> u32 {
    let args: Vec<String> = env::args().collect();

    let port: u32 = match args[2].parse() {
        Ok(port) => port,
        Err(_) => {
            eprintln!("Error: El argumento port debe ser un número entero.");
            std::process::exit(1);
        }
    };

    port
}

pub(crate) fn get_other_processes_filename() -> String {
    let args: Vec<String> = env::args().collect();
    args[3].clone()
}

pub(crate) fn get_other_processes(filepath: &Path) -> Vec<Process> {
    let mut other_processes: Vec<Process> = Vec::new();
    let reader = file_handler::get_reader_for_file_or_kill_process(filepath);

    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => {
                eprintln!("Error: No se pudo leer una línea del archivo de procesos.");
                std::process::exit(1);
            }
        };
        let parts: Vec<&str> = line.split(";").collect();

        if parts.len() != 3 {
            eprintln!("Error: Cada línea del archivo de procesos debe tener 3 partes separadas por ';'.");
            std::process::exit(1);
        }

        let id: u32 = match parts[0].parse() {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Error: El ID del proceso debe ser un número entero.");
                std::process::exit(1);
            }
        };

        let ip = parts[1].to_string();

        let port: u32 = match parts[2].parse() {
            Ok(port) => port,
            Err(_) => {
                eprintln!("Error: El puerto del proceso debe ser un número entero.");
                std::process::exit(1);
            }
        };
        let leader = false;

        other_processes.push(Process { id, ip, port, leader });
    }

    other_processes
}

pub(crate) fn check_pid_and_port(pid: u32, port: u32, other_processes: &Vec<Process>) {
    for process in other_processes.iter() {
        if process.id == pid {
            eprintln!("Error: El ID del proceso debe ser único.");
            std::process::exit(1);
        }

        // ? Si se corre localmente, el puerto debe ser único. Si no, comentar esta validacion.
        if process.port == port {
            eprintln!("Error: El puerto del proceso debe ser único cuando se corre localmente.");
            std::process::exit(1);
        }
    }
}