use std::path::Path;

fn open_file_or_kill_process(filepath: &Path) -> std::fs::File {
    match std::fs::File::open(filepath) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("Error: No se pudo abrir el archivo {}.", filepath.display());
            std::process::exit(1);
        }
    }
}

pub(crate) fn get_reader_for_file_or_kill_process(filepath: &Path) -> std::io::BufReader<std::fs::File> {
    let file = open_file_or_kill_process(filepath);
    std::io::BufReader::new(file)
}

