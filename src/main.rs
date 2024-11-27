mod utils;
mod listener;
mod process;
mod procceses_list_handler;
mod healthchecker;
mod election;
mod consts;
mod work_thread;

use utils::arg_handler;
use utils::file_handler;
use arg_handler::{check_args, get_process_id, get_process_port, get_other_processes_filename, get_other_processes, check_pid_and_port};
use std::sync::{Arc, RwLock};
use std::sync::mpsc::channel;
use crate::listener::listen_for_process_messages;
use crate::process::Process;

fn push_me(mut other_processes: Vec<Process>, pid: u32, port: u32) -> Vec<Process> {
    other_processes.push(Process {
        id: pid,
        ip: "0.0.0.0".to_string(),
        port,
        leader: false,
        me: true
    });

    other_processes
}

fn main() {
// * Armado de la lista de procesos
    check_args();
    let other_processes_filepath = get_other_processes_filename();
    let mut other_processes = get_other_processes(other_processes_filepath.as_ref());

    let pid = get_process_id();
    let port = get_process_port();
    check_pid_and_port(pid, port, &other_processes);
    let work_port; //TODO Recibir puerto de thread work por argumento

    println!("Iniciando proceso con ID: {} y PORT: {}", pid, port);
    other_processes = push_me(other_processes, pid, port);

// * Alocamos los recursos para poder iniciar los threads de liderazgo y subordinacion
    // ? los tx trasmiten al thread de election (son para los threads heartbeat y listener), el rx recibe de los threads de election y heartbeat
    let (tx_election_thread, rx_heartbeat_listener_thread) = channel();
    let tx_election_thread1 = tx_election_thread.clone();

    // ? los tx trasmiten al thread de process_handler, el rx recibe de los threads de election y listener
    let (tx_process_handler, rx_election_listener_thread) = channel();
    let tx_process_handler1 = tx_process_handler.clone();

    // ? el tx trasmite al thread de heartbeat, el rx recibe del thread listener
    let (tx_heartbeat_thread, rx_listener_thread) = channel();

    // ? mantenemos referencias de lectura para que los threads puedan saber que procesos hay, sus datos y quien es el lider
    let other_processes_mutex = Arc::new(RwLock::new(other_processes));
    let other_processes1_read_ref = Arc::clone(&other_processes_mutex);
    let other_processes2_read_ref = Arc::clone(&other_processes_mutex);
    let other_processes3_read_ref = Arc::clone(&other_processes_mutex);

    //TODO Considerar si es necesario conocer que proceso es lider. Quizas no es necesario y se puede sacar el thread de process_handler para simplificar.
// * Iniciamos los threads de liderazgo y subordinacion
    // ? iniciamos el thread que gestionara el estado de la lista de procesos. Puede recibir mensajes de:
    //   * listener thread: "new leader {pid}": indica que el proceso con pid es el nuevo lider
    //   * election thread: "new leader {pid}": indica que el proceso con pid es el nuevo lider
    let process_list_handler = procceses_list_handler::start_process_list_handling(other_processes_mutex, rx_election_listener_thread);

    // ? iniciamos el thread que escuchara y gestionara los mensajes de otros nodos. Se comunica con:
    //   * election thread: "ELECTION": indica que se debe iniciar un proceso de eleccion
    //   * heartbeat thread: "HEARTBEAT": indica que se recibio un heartbeat
    //   * process list handler: "new leader {pid}": indica que el proceso con pid es el nuevo lider
    let listener_thread_handler = listen_for_process_messages(port, tx_process_handler, tx_heartbeat_thread, tx_election_thread);

    // ? iniciamos el thread que maneja los heartbeats (enviando o esperando recibirlos segun el rol del proceso). Se comunica con:
    //   * election thread: "ELECTION": indica que se debe iniciar un proceso de eleccion
    // ? recibe mensajes de:
    //   * listener thread: "HEARTBEAT": indica que se recibio un heartbeat
    let heartbeat_thread_handler = healthchecker::start_healthcheck_thread(rx_listener_thread, tx_election_thread1, other_processes1_read_ref);

    // ? iniciamos el thread de eleccion de lider. Se comunica con:
    //   * process list handler: "new leader {pid}": indica que el proceso con pid es el nuevo lider
    // ? recibe mensajes de:
    //   * listener thread: "ELECTION": indica que se debe iniciar un proceso de eleccion
    //   * heartbeat thread: "ELECTION": indica que se debe iniciar un proceso de eleccion
    let election_thread_handler = election::start_election_thread(other_processes2_read_ref, rx_heartbeat_listener_thread, tx_process_handler1);

    // ? iniciamos el thread de trabajo, que se encarga de comportarse como subordinado o lider segun corresponda. Esto lo sabe por el estado de la lista de procesos.
    let work_thread_handler = work_thread::start_work_thread(other_processes3_read_ref);

// * Espera de que los otros threads se cierren
    match listener_thread_handler.join(){
        Ok(_) => {},
        Err(_) => { eprintln!("Error: El hilo de manejo de mensajes terminó inesperadamente"); }
    }

    match process_list_handler.join(){
        Ok(_) => {},
        Err(_) => { eprintln!("Error: El hilo de manejo de la lista de procesos terminó inesperadamente"); }
    }

    match heartbeat_thread_handler.join(){
        Ok(_) => {},
        Err(_) => { eprintln!("Error: El hilo de eleccion terminó inesperadamente"); }
    }

    match election_thread_handler.join(){
        Ok(_) => {},
        Err(_) => { eprintln!("Error: El hilo de eleccion terminó inesperadamente"); }
    }

    match work_thread_handler.join(){
        Ok(_) => {},
        Err(_) => { eprintln!("Error: El hilo de trabajo terminó inesperadamente"); }
    }
}