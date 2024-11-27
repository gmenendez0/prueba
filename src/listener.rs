use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::thread::JoinHandle;
use crate::process::Process;
use crate::utils::tcp::get_tcp_listener_or_kill_process;
use crate::consts::{NEW_LIDER_MSG, HEARTBEAT_MSG, START_ELECTION_MSG, ELECTION_MSG, NEW_LEADER_ANSWER};


fn get_me(processes: &Vec<Process>) -> Result<&Process, String> {
    for process in processes {
        if process.me {
            return Ok(process);
        }
    }

    Err("No se encontro el proceso actual en la lista de procesos".to_string())
}

fn am_i_leader(processes: &Vec<Process>) -> bool {
    let me = match get_me(processes){
        Ok(me) => me,
        Err(e) => {
            eprintln!("{}", e);
            return false;
        }
    };

    me.leader
}

pub(crate) fn listen_for_process_messages(port: u32, mut process_handler_tx: std::sync::mpsc::Sender<String>, mut heartbeat_tx: std::sync::mpsc::Sender<String>, mut election_tx: std::sync::mpsc::Sender<String>) -> JoinHandle<()>{
    // ? abre el socket para que otros puedan comunicarse
    let listener = get_tcp_listener_or_kill_process(port);

    thread::spawn(move || {
        // ? escucha las conexiones entrantes
        println!("Escuchando conexiones en el puerto {}", port);
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    // ? para cada conexion, maneja el mensaje
                    handle_node_message(stream, &mut process_handler_tx, &mut heartbeat_tx, &mut election_tx);
                }
                Err(e) => {
                    eprintln!("Error al aceptar conexión: {}", e)
                },
            }
        }
    })
}

fn handle_node_message(mut stream: TcpStream, tx: &mut std::sync::mpsc::Sender<String>, tx_heartbeat: &mut std::sync::mpsc::Sender<String>, election_tx: &mut std::sync::mpsc::Sender<String>){
    // ? convierte el mensaje a un string
    let mut buffer = [0; 1024];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(bytes_read) => bytes_read,
        Err(e) => {
            eprintln!("Error al leer mensaje: {}", e);
            return;
        }
    };
    let message = String::from_utf8_lossy(&buffer[..bytes_read]);

    // ? obtiene la respuesta a enviar
    let answer = process_message(&message, tx, tx_heartbeat);

    // ? envia la respuesta
    match stream.write(answer.as_bytes()) {
        Ok(_) => {},
        Err(e) => eprintln!("Error al enviar respuesta: {}", e)
    }

    // ? chequeo si la rta que devuelvo corresponde a que tengo que detonar una eleccion
    if answer == ELECTION_MSG {
        // ? envio mensaje de solicitud de inicio de eleccion
        match election_tx.send(START_ELECTION_MSG.to_string()) {
            Ok(_) => {},
            Err(e) => eprintln!("Error al enviar mensaje de eleccion: {}", e)
        }
    } else if answer == "error"{
        eprintln!("Error al procesar mensaje: {}", message);
    }
}

// ? debe mejorarse el manejo de errores
pub(crate) fn process_message(message: &str, tx: &mut std::sync::mpsc::Sender<String>, tx_heartbeat: &mut std::sync::mpsc::Sender<String>) -> String {
    println!("Mensaje recibido: {}", message);

    if message == START_ELECTION_MSG {
        ELECTION_MSG.to_string()
    } else if message.starts_with(NEW_LIDER_MSG) {
        return match tx.send(message.to_string()) {
            Ok(_) => NEW_LEADER_ANSWER.to_string(),
            Err(e) => {
                eprintln!("Error al enviar mensaje: {}", e);
                "error".to_string()
            }
        }
    } else if message == HEARTBEAT_MSG {
        match tx_heartbeat.send(HEARTBEAT_MSG.to_string()) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Error al enviar mensaje de heartbeat: {}", e);
                return "error".to_string();
            }
        }
        "ok".to_string()
    } else {
        println!("Mensaje desconocido: {}", message);
        "error".to_string()
    }
}