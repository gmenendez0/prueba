use std::io::Write;
use std::process::exit;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use crate::consts::{HEARTBEAT_MSG, START_ELECTION_MSG};
use crate::process::Process;
use crate::utils::tcp::get_server_connection;

fn i_am_leader(processes: &Arc<RwLock<Vec<Process>>>) -> bool {
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


pub fn start_healthcheck_thread(mut rx: std::sync::mpsc::Receiver<String>, mut election_tx: std::sync::mpsc::Sender<String>, other_processes: Arc<RwLock<Vec<Process>>>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut last_heartbeat_time = Instant::now();
        let timeout = Duration::from_secs(60);

        loop {
            if i_am_leader(&other_processes) {
                // ? Si soy lider, envio heartbeat a los demas procesos
                send_heartbeat(&other_processes);
            } else {
                // ? Si no soy lider, chequeo si recibi heartbeat
                check_for_heartbeat(&mut rx, &mut last_heartbeat_time, timeout, &mut election_tx);
            }

            // ? Espero 10 segundos antes de volver a actuar
            thread::sleep(Duration::from_millis(10000));
        }
    })
}

pub fn check_for_heartbeat(rx: &mut std::sync::mpsc::Receiver<String>, last_heartbeat_time: &mut Instant, timeout: Duration, election_tx: &mut std::sync::mpsc::Sender<String>) {
    println!("Chequeando si recibi heartbeat...");

    // ? intento recibir un mensaje del canal.
    match rx.try_recv() {
        Ok(message) => {
            // ? si hay un mensaje de heartbeat, actualizo la ultima vez que recibi un heartbeat.
            if message == HEARTBEAT_MSG {
                println!("Heartbeat recibido, actualizando el temporizador.");
                *last_heartbeat_time = Instant::now();
            } else {
                println!("Mensaje inesperado en el canal: {}", message);
            }
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            // ? si no hay mensaje, chequeo si paso el tiempo de timeout.
            if last_heartbeat_time.elapsed() > timeout {
                // ? si paso tiempo de timeout, envio un mensaje al hilo de eleccion para que inicie un proceso de eleccion.
                println!("No se recibió heartbeat en el tiempo esperado. TIMEOUT. Iniciando elección de líder...");

                match election_tx.send(START_ELECTION_MSG.to_string()) {
                    Ok(_) => println!("Mensaje enviado al hilo de elección."),
                    Err(e) => eprintln!("Error al enviar mensaje al hilo de elección: {}", e)
                }

                *last_heartbeat_time = Instant::now();
            }
        }
        Err(_) => {
            eprintln!("Error desconocido al recibir mensaje del canal.");
            exit(1); //TODO manejar error
        }
    }
}

pub fn send_heartbeat(other_processes: &Arc<RwLock<Vec<Process>>>) {
    println!("Enviando heartbeat a los demas procesos...");

    let processes_guard = match other_processes.read() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Error al obtener el guard de procesos: {}", e);
            return;
        }
    };

    //TODO Manejar el caso de que no se pueda enviar un heartbeat a un proceso. Definir timeouts
    // ? para cada uno de los procesos que no son yo y no son lider (si llego aca siempre yo y el lider somos uno)
    for process in processes_guard.iter() {
        if !process.leader && !process.me {
            // ? determino la direccion del proceso
            let addr = format!("{}:{}", process.ip, process.port);

            // ? me conecto
            let mut conn = match get_server_connection(&addr) {
                Ok(conn) => conn,
                Err(e) => {
                    eprintln!("Error enviando heartbeat a {}: {}", process.id, e);
                    continue;
                }
            };

            // ? envio el mensaje de heartbeat
            let msg = HEARTBEAT_MSG.to_string();
            match conn.write(msg.as_bytes()) {
                Ok(_) => println!("Mensaje enviado a {}", addr),
                Err(e) => eprintln!("Error al enviar mensaje a {}: {}", addr, e)
            }
        }
    }
}