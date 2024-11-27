mod utils;
mod listener;
mod process;
mod procceses_list_handler;
mod healthchecker;
mod election;
mod consts;

use utils::arg_handler;
use utils::file_handler;
use arg_handler::{check_args, get_process_id, get_process_port, get_other_processes_filename, get_other_processes, check_pid_and_port};
use std::process::exit;
use std::sync::{Arc, RwLock};
use std::sync::mpsc::channel;
use crate::listener::listen_for_process_messages;
use crate::process::Process;

/*
1. Proceso inicia. Arma su lista de procesos, y luego se incluye a si mismo.
    1.1. Levanta un hilo de election donde lleva el tiempo desde que se recibio el ultimo heartbeat, (al principio es el tiempo de inicio del proceso), y si este tiempo excede el timeout, inicia una eleccion.
    1.2. Levanta un hilo donde escucha solicitudes de otros nodos.
2. Se pone a escuchar mensajes de otros procesos:
- Si hay un lider, le llegara un heartbeat tarde o temprano.
- Si no hay un lider, no llegara ningun heartbeat, por lo que el hilo de election iniciara una eleccion.
3. Inicio de eleccion:
- Se envia un mensaje "lider?" a todos los procesos. Si alguno responde que es lider, se convierte en subordinado.
- Si ninguno responde, se autoproclama lider y envia un mensaje "nuevo lider {pid}" a todos los procesos.
- Si ninguno responde que si, pero si que no, inicia un proceso de eleccion.
 */

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

// * Cierra los channels

// * Espera de que los otros threads se cierren
    match listener_thread_handler.join(){
        Ok(_) => {},
        Err(_) => {
            eprintln!("Error: El hilo de manejo de mensajes terminó inesperadamente");
            exit(1);
        }
    }

    match process_list_handler.join(){
        Ok(_) => {},
        Err(_) => {
            eprintln!("Error: El hilo de manejo de la lista de procesos terminó inesperadamente");
            exit(1);
        }
    }

    match heartbeat_thread_handler.join(){
        Ok(_) => {},
        Err(_) => {
            eprintln!("Error: El hilo de eleccion terminó inesperadamente");
            exit(1);
        }
    }

    match election_thread_handler.join(){
        Ok(_) => {},
        Err(_) => {
            eprintln!("Error: El hilo de eleccion terminó inesperadamente");
            exit(1);
        }
    }
}

/*
ALGORITMO DE ELECCION DE LIDER BULLY:
1. Cada proceso conoce su ID y el ID de los procesos con los que puede comunicarse, junto con su IP y su puerto.
2. Cada proceso tiene un estado que puede ser: SUBORDINATE, COORDINATOR.
3. Cuando un proceso inicia, debe averiguar si ya hay un lider o no.
    - Para ello, se comunica con cada uno de los procesos con los que puede comunicarse y les pregunta si son lider.
    A. Si un proceso responde que es lider, el proceso que pregunta se convierte en subordinado.
    B. Si ningun proceso responde que es lider, entonces arranca un proceso de eleccion.
    C. Si no recibe respuesta de ningun tipo de ningun proceso, el proceso que inicio es el unico, entonces se autoproclama lider.
4. Para iniciar un proceso de eleccion, el proceso que inicia la eleccion envia un mensaje de eleccion a todos los procesos con ID mayor a el.
5. Si algún proceso con un ID más alto responde con "OK", significa que ese proceso tomará el control de la elección.
   El proceso queda en espera de que llegue un mensaje "Nuevo líder".
6. Si no recibe respuesta de ningun tipo de ningun proceso en X tiempo, el proceso que inicio la eleccion es el nuevo lider.

RECEPCION DE "ELECTION" Y ENVIO DE OK:
1. Cuando un proceso P recibe un mensaje de eleccion, responde con "OK" si su ID es mayor al del proceso que inicio la eleccion.
2. Si P no tiene procesos con ID mayor a el, entonces se autoproclama lider.
3. Si P tiene procesos con ID mayor al de el, P envia un mensaje de eleccion a todos los procesos con ID mayor a el.

PROCLAMACION DE LIDER:
1. Un proceso que se autoproclama lider envia un mensaje "Nuevo lider {pid}" a todos los procesos con los que puede comunicarse.
2. Los procesos que reciben el mensaje actualizan su estado a subordinado y guardan el ID del nuevo lider.
3. El proceso que se autoproclama lider actualiza su estado a COORDINATOR.
 */