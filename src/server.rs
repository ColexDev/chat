use std::io::{Read, Write, BufReader};
use std::net::{TcpListener, TcpStream};
use std::str;
use std::thread;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::path::Path;
use std::time;
extern crate time as time_crate;

const IP: &str = "127.0.0.1:6001";

fn lines_from_file(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("no such file");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

fn get_file_size(filename: impl AsRef<Path>) -> u64 {
    let file = File::open(filename).expect("no such file");
    let metadata = file.metadata().expect("F");
    metadata.len()
}

fn handle_client(mut socket: TcpStream) {
    loop {
        let mut data = [0 as u8; 500];
        match socket.read(&mut data) {
            Ok(num_bytes) => {
                if data[0] == 0 {
                    return;
                }
                let mut fd = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open("messages.txt")
                    .unwrap();
                let buf = String::from_utf8_lossy(&data);

                // FIX: Append the file length to EVERY message as a header, this needs to be done
                // FIX: so that we don't get the int parsing error

                if &buf[0..num_bytes] == "$_GET_FILE" {
                    let file_length = get_file_size("messages.txt");
                    let lines = lines_from_file("messages.txt");
                    let mut num_lines = 0;
                    for _line in &lines {
                        num_lines += 1;
                    }
                    let total_length = (file_length.to_string().len() * num_lines) + file_length as usize;
                    for line in lines {
                        let mut send = String::new();
                        send.push_str(&total_length.to_string());
                        send.push_str(" ");
                        send.push_str(&line);
                        send.push_str("\n");
                        socket.write(send.as_bytes()).expect("f");
                        println!("SENDING: {}", send);
                    }
                } else {
                    let buf = String::from_utf8_lossy(&data);
                    let mut send = String::new();
                    let formatted_time = time_crate::strftime("%R", &time_crate::now()).expect("F");
                    send.push_str(&formatted_time);
                    send.push_str(" | ");
                    send.push_str(&buf);
                    send.push_str("\n");
                    fd.write(&send[0..(num_bytes + 8)].as_bytes()).expect("Failed to write to file");
                }
                // println!("{}", buf);
            }
            Err(e) => {
                // NOTE: This seems to also handle the return if statement I have in the first line of Ok
                eprintln!("Error: {}", e);
            }
        }
    }
}

// fn write_message(mut stream: &TcpStream, message: &[u8]) {
//     stream
//         .write(message)
//         .expect("failed to write to all clients");
// }

fn main() {
    let server = TcpListener::bind(IP).expect("Listener failed to bind");

    //TODO: Have clients request the whole message file every n ms instead of
    //TODO: having them be forcefully sent it, this will make it easier as
    //TODO: when handle_client recieved the request, it can just send the message file
    loop {
        if let Ok((socket, _)) = server.accept() {
            thread::spawn(|| handle_client(socket));
        }
    }
}
