use std::io::{Read, Write};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use crate::consts::{NEW_LIDER_MSG, START_ELECTION_MSG};
use crate::process::Process;
use crate::utils::tcp::get_server_connection;

pub(crate) fn start_election_thread(processes: Arc<RwLock<Vec<Process>>>, rx: std::sync::mpsc::Receiver<String>, mut tx: std::sync::mpsc::Sender<String>) -> JoinHandle<()> {
    thread::spawn(move || {
        for msg in rx {
            if msg == START_ELECTION_MSG {
                start_election(&processes, &mut tx);
            }
        }
    })
}

fn get_my_id(processes: &Arc<RwLock<Vec<Process>>>) -> Result<u32, String> {
    let processes_guard = match processes.read() {
        Ok(guard) => guard,
        Err(e) => return Err(format!("Error al obtener el guard de procesos: {}", e))
    };

    for process in processes_guard.iter() {
        if process.me {
            return Ok(process.id);
        }
    }

    Err("No se encontro el proceso actual en la lista de procesos".to_string())
}

fn get_addr_for_process(process: &Process) -> String {
    format!("{}:{}", process.ip, process.port)
}

pub(crate) fn start_election(processes: &Arc<RwLock<Vec<Process>>>, tx: &mut std::sync::mpsc::Sender<String>) {
    let mut i_am_leader = false;
    println!("Iniciando eleccion de lider...");

    let my_id = match get_my_id(processes) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{}", e);
            return; //TODO
        }
    };

    let mut answers = 0;
    let processes_guard = match processes.read() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Error al obtener el guard de procesos: {}", e);
            return; //TODO
        }
    };

    for process in processes_guard.iter() {
        if process.id > my_id {
            // 1. Obtengo la dirección del proceso
            let addr = get_addr_for_process(process);
            println!("Enviando mensaje de ELECTION a {}", addr);

            // 2. Me conecto y le aviso de la elección
            let mut conn = match get_server_connection(&addr) {
                Ok(conn) => conn,
                Err(e) => {
                    eprintln!("{}", e);
                    return; //TODO
                }
            };

            // 3. Configuro el timeout para la lectura
            let timeout = Duration::from_secs(5);
            if let Err(e) = conn.set_read_timeout(Some(timeout)) {
                eprintln!("Error al configurar timeout en la conexión: {}", e);
                return; //TODO
            }

            // TODO Chequear que pasa si no puede escribir
            // 4. Envío el mensaje de ELECTION
            match conn.write(START_ELECTION_MSG.as_bytes()) {
                Ok(_) => println!("Mensaje ELECTION enviado a {}", addr),
                Err(e) => {
                    eprintln!("Error al enviar mensaje de ELECTION: {}", e);
                    return; //TODO
                }
            }

            // 5. Espero respuesta o timeout
            let mut buffer = [0; 1024];
            match conn.read(&mut buffer) {
                Ok(bytes_read) => {
                    let response = String::from_utf8_lossy(&buffer[..bytes_read]);
                    println!("Respuesta recibida de {}: {}", addr, response);
                    answers = answers + 1;
                }
                Err(e) => {
                    eprintln!("Error o timeout esperando respuesta de {}: {}", addr, e);
                }
            }
        }
    }

    if answers == 0 {
        println!("No se recibieron respuestas. Autoproclamandose líder...");
        let msg = format!("{} {}", NEW_LIDER_MSG, my_id);

        // ? aviso al hilo que maneja los procesos que hay un nuevo lider, yo
        match tx.send(msg.clone()) {
            Ok(_) => println!("Me setee como lider. Avisando al resto"),
            Err(e) => {
                eprintln!("Error al enviar mensaje: {}", e);
                return; //TODO
            }
        }

        // ? aviso al resto de los procesos que hay un nuevo lider, yo
        let processes_guard = match processes.read() {
            Ok(guard) => guard,
            Err(e) => {
                eprintln!("Error al obtener el guard de procesos: {}", e);
                return; //TODO
            }
        };

        for process in processes_guard.iter() {
            if process.id != my_id {
                println!("Enviando mensaje de nuevo lider a {}", process.id);

                let addr = get_addr_for_process(process);
                let mut conn = match get_server_connection(&addr) {
                    Ok(conn) => conn,
                    Err(e) => {
                        eprintln!("Error enviando mensaje de nuevo lider a {}: {}", process.id, e);
                        continue;
                    }
                };

                match conn.write(msg.as_bytes()) {
                    Ok(_) => println!("Mensaje enviado a {}", addr),
                    Err(e) => {
                        eprintln!("Error al enviar mensaje a {}: {}", addr, e);
                    }
                }
            }
        }

        i_am_leader = true;
    }

    // ? si soy el lider, inicio las tareas de lider y me quedo esperando a que se cierre ese thread. Es valido porque el modulo election no se volvera a usar en un lider, entonces si se bloquea da igual.
    if i_am_leader {
        println!("Proceso de eleccion de lider finalizado. Iniciando tareas de lider...");
        let leader_main_handle = thread::spawn(move || {
            //TODO Lanzar nuevo thread para tareas de lider
            todo!("iniciar tareas de lider")
        });

        match leader_main_handle.join(){
            Ok(_) => {},
            Err(_) => {
                eprintln!("Error: El hilo de lider terminó inesperadamente");
            }
        }
    }
}