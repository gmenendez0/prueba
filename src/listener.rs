use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::thread::JoinHandle;
use crate::process::Process;
use crate::utils::tcp::get_tcp_listener_or_kill_process;
use crate::utils::tcp::get_peer_addr;
use crate::election::start_election;


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

pub(crate) fn listen_for_process_messages(port: u32, mut rx: std::sync::mpsc::Receiver<String>, mut tx: std::sync::mpsc::Sender<String>, mut tx_heartbeat: std::sync::mpsc::Sender<String>, other_processes: &Vec<Process>) -> JoinHandle<()>{
    let listener = get_tcp_listener_or_kill_process(port);

    thread::spawn(move || {
        println!("Escuchando conexiones en el puerto {}", port);
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let peer_addr = get_peer_addr(&stream).unwrap_or("UNABLE TO GET PEER ADDRESS".to_string());
                    println!("Nueva conexión: {}", peer_addr);

                    handle_node_message(stream, &mut rx, &mut tx, &mut tx_heartbeat, other_processes);
                }
                Err(e) => {
                    eprintln!("Error al aceptar conexión: {}", e)
                },
            }
        }
    })
}

fn handle_node_message(mut stream: TcpStream, rx: &mut std::sync::mpsc::Receiver<String>, tx: &mut std::sync::mpsc::Sender<String>, tx_heartbeat: &mut std::sync::mpsc::Sender<String>, other_processes: &Vec<Process>){
    let mut buffer = [0; 1024];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(bytes_read) => bytes_read,
        Err(e) => {
            eprintln!("Error al leer mensaje: {}", e);
            return;
        }
    };

    let message = String::from_utf8_lossy(&buffer[..bytes_read]);
    let answer = process_message(&message, rx, tx, tx_heartbeat, other_processes);

    match stream.write(answer.as_bytes()) {
        Ok(_) => {},
        Err(e) => eprintln!("Error al enviar respuesta: {}", e)
    }

    if answer == "ok election" {
        start_election(other_processes, tx.clone()); // Esto deberia hacerse en otro thread que fue creado desde el main, se debe comunicar via un channel.
    } else if answer == "error"{
        eprintln!("Error al procesar mensaje: {}", message);
    }
}

// ? debe mejorarse el manejo de errores
pub(crate) fn process_message(message: &str, rx: &mut std::sync::mpsc::Receiver<String>, tx: &mut std::sync::mpsc::Sender<String>, tx_heartbeat: &mut std::sync::mpsc::Sender<String>, other_processes: &Vec<Process>) -> String {
    println!("Mensaje recibido: {}", message);

    if message == "ELECTION" {
        "ok election".to_string()
    } else if message.starts_with("new leader") {
        return match tx.send(message.to_string()) {
            Ok(_) => "ok".to_string(),
            Err(e) => {
                eprintln!("Error al enviar mensaje: {}", e);
                "error".to_string()
            }
        }
    } else if message == "HEARTBEAT"{
        match tx_heartbeat.send("HEARTBEAT".to_string()) {
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