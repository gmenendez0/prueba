use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
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

/// Establish a TCP connection to a server.
///
/// This function takes a string `address` representing the server's address and attempts
/// to establish a TCP connection. It returns a `Result` containing a `TcpStream` if the
/// connection is successful or an error message as a `String` if an error occurs.
///
/// # Arguments
/// - `address`: A string representing the server's address (e.g., "127.0.0.1:8080").
///
/// # Returns
/// Returns a `Result` containing a `TcpStream` if the connection is successful, or a
/// `String` with an error message if an error occurs.
///
/// # Errors
/// Returns an error message as a `String` if there is an issue connecting to the server.
pub fn get_server_connection(address: &str) -> Result<TcpStream, String> {
    match TcpStream::connect(address) {
        Ok(stream) => Ok(stream),
        Err(error) => Err(format!("Error connecting to server: {}", error)),
    }
}

/// Write bytes to a stream.
///
/// This function takes a mutable reference to a `Write` trait object (`stream`) and
/// a slice of bytes (`message`). It writes the bytes to the stream and returns a
/// `Result` indicating success or an error message as a `String` if an error occurs.
///
/// # Arguments
/// - `stream`: A mutable reference to a `Write` trait object, allowing writing bytes.
/// - `message`: A slice of bytes to be written to the stream.
///
/// # Returns
/// Returns `Ok(())` if the write operation is successful, or a `String` with an error
/// message if an error occurs during the write operation.
///
/// # Errors
/// Returns an error message as a `String` if there is an issue writing bytes to the stream.
pub fn write_bytes_to_stream(stream: &mut dyn Write, message: &[u8]) -> Result<(), String> {
    match stream.write(message) {
        Ok(_) => Ok(()),
        Err(error) => Err(format!("Error sending message: {}", error)),
    }
}

/// Read a response from a stream and convert it to a UTF-8 encoded string.
///
/// This function takes a mutable reference to a `Read` trait object (`stream`) and
/// reads the response as a UTF-8 encoded string. It returns a `Result` containing the
/// string response if successful, or an error message as a `String` if an error occurs.
///
/// # Arguments
/// - `stream`: A mutable reference to a `Read` trait object, allowing reading bytes.
///
/// # Returns
/// Returns a `Result` containing a string response if the read and conversion operations
/// are successful, or a `String` with an error message if an error occurs.
///
/// # Errors
///
/// Returns an error message as a `String` if there is an issue reading from the stream
/// or converting the response to a UTF-8 encoded string.
pub fn get_response_from_server_as_string(stream: &mut dyn Read) -> Result<String, String> {
    let response_buffer = get_response_from_server_as_u8_buffer(stream)?;

    match String::from_utf8(response_buffer.to_vec()) {
        Ok(response) => Ok(response),
        Err(error) => Err(format!("Error converting response to string: {}", error)),
    }
}

/// Read a response from a stream into a fixed-size buffer of u8 bytes.
///
/// This function takes a mutable reference to a `Read` trait object (`stream`) and
/// reads the response into a fixed-size buffer of u8 bytes. It returns a `Result`
/// containing the buffer if the read operation is successful, or an error message
/// as a `String` if an error occurs.
///
/// # Arguments
/// - `stream`: A mutable reference to a `Read` trait object, allowing reading bytes.
///
/// # Returns
/// Returns a `Result` containing a fixed-size buffer of u8 bytes if the read
/// operation is successful, or a `String` with an error message if an error occurs.
///
/// # Errors
/// Returns an error message as a `String` if there is an issue reading from the stream.
pub fn get_response_from_server_as_u8_buffer(stream: &mut dyn Read) -> Result<[u8; 16384], String> {
    let mut response_buffer = [0u8; 16384];

    if stream.read(&mut response_buffer).is_err() {
        return Err("Error reading response from server.".to_string());
    }

    Ok(response_buffer)
}

pub fn get_peer_addr(stream: &TcpStream) -> Result<String, String> {
    match stream.peer_addr() {
        Ok(addr) => Ok(addr.to_string()),
        Err(e) => Err(format!("Error al obtener dirección del peer: {}", e)),
    }
}