use std::sync::{Arc, RwLock};
use std::thread;
use crate::process::Process;
use crate::consts::NEW_LIDER_MSG;

fn print_processes(processes: &Arc<RwLock<Vec<Process>>>) {
    let processes_guard = match processes.read(){
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Error al obtener el guard de procesos: {}", e);
            return;
        }
    };

    for process in processes_guard.iter() {
        println!("ID: {}, IP: {}, PORT: {}", process.id, process.ip, process.port);
    }
}

// ? recibe mensajes del thread de election y de listener que avisan de nuevos lideres
pub(crate) fn start_process_list_handling(processes: Arc<RwLock<Vec<Process>>>, rx: std::sync::mpsc::Receiver<String>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        print_processes(&processes);

        for msg in rx {
            // ? Llega un mensaje que avisa que hay un nuevo lider
            if msg.starts_with(NEW_LIDER_MSG) {
                // ? obtenemos su id
                let new_leader_id = msg.split_whitespace().collect::<Vec<&str>>()[2];
                let id = match new_leader_id.parse::<u32>() {
                    Ok(id) => id,
                    Err(e) => {
                        eprintln!("Error al parsear ID de nuevo lider: {}", e);
                        continue;
                    }
                };

                // ? marcamos al nuevo lider y a los demas como no lider
                let mut processes_guard = match processes.write() {
                    Ok(processes) => processes,
                    Err(e) => {
                        eprintln!("Error al obtener el guard de procesos: {}", e);
                        continue;
                    }
                };

                for process in processes_guard.iter_mut() {
                    if process.id == id {
                        process.leader = true;
                    } else {
                        process.leader = false;
                    }
                }
            }
        }
    })
}