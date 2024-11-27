use std::sync::{Arc, RwLock};
use crate::process::Process;

pub(crate) fn start_work_thread(processes: Arc<RwLock<Vec<Process>>>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        println!("Iniciando hilo de trabajo...");
        start_work(processes);
    })
}

fn am_i_leader(processes: &Arc<RwLock<Vec<Process>>>) -> bool {
    let processes_guard = match processes.read() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Error al obtener el guard de procesos: {}", e);
            return false;
        }
    };

    for process in processes_guard.iter() {
        if process.leader && process.me {
            return true;
        }
    }

    false
}

fn start_work(processes: Arc<RwLock<Vec<Process>>>) {
    /*
        TODO iniciar estructuras
        TODO Abrir listener. Por cada mensaje del listener, preguntar si soy lider o no y manejar mensaje de una manera u otra.

        mensaje -> listener(5050) -> soyLider? -> si -> intento procesarlo como un trip
        mensaje -> listener(5050) -> soyLider? -> no -> intento procesarlo como una actualizacion de estado

        nodo_1 -> dirA LIDER
        nodo_2 -> dirB Sub
        nodo_3 -> dirC sub
    */

    loop {
        if am_i_leader(&processes) {
            //TODO Realizo tarea como lider
        } else {
            //TODO Realizo tarea como subordinado
        }
    }
}