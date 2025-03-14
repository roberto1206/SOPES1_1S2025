use std::process::{Command, Stdio};
use std::io::Write;

fn main() {
    let cronjob = "\
* * * * * /bin/bash /home/roberto/Documentos/sopes1/SOPES1_1S2025/PROYECTO1/sscripts/script.sh
* * * * * sleep 30 && /bin/bash /home/roberto/Documentos/sopes1/SOPES1_1S2025/PROYECTO1/sscripts/script.sh

";

    println!("Actualizando crontab...");
    let mut child = Command::new("crontab")
        .stdin(Stdio::piped())
        .spawn()
        .expect("No se pudo iniciar el proceso crontab");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(cronjob.as_bytes()).expect("Error al escribir en crontab");
    }

    let status = child.wait().expect("Error al esperar el proceso crontab");

    if status.success() {
        println!("Crontab actualizado correctamente");
    } else {
        println!("Error al actualizar crontab");
    }

    // Verificar que el crontab está correctamente instalado
    println!("Verificando crontab...");
    let output = Command::new("crontab")
        .arg("-l")
        .output()
        .expect("Error al ejecutar crontab -l");

    println!("Salida estándar:\n{}", String::from_utf8_lossy(&output.stdout));
    println!("Salida de error:\n{}", String::from_utf8_lossy(&output.stderr));
}
