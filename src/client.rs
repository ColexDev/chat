extern crate ncurses;

use ncurses::*;
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::thread;
use std::time;

const KEY_I: i32 = 105;
const KEY_J: i32 = 106;
const KEY_K: i32 = 107;
// const KEY_L: i32 = 108;
const KEY_Q: i32 = 113;
const KEY_ESC: i32 = 27;
const KEY_ENTER: i32 = 10;
const KEY_BACKSPACE: i32 = 127;

struct ClientState<'a> {
    message_win: WINDOW,
    typing_win: WINDOW,
    offset: i32,
    max_x: i32,
    max_y: i32,
    username: &'a String,
    // ip: &'a String,
    // socket: &TcpStream,
}

struct Buffer {
    line: String,
    cursor: i32,
}

impl Buffer {
    fn new() -> Buffer {
        Buffer {
            line: "".to_string(),
            cursor: 0,
        }
    }
}

fn create_win(height: i32, width: i32, start_y: i32, start_x: i32) -> WINDOW {
    let win = newwin(height, width, start_y, start_x);
    box_(win, 0, 0);
    wrefresh(win);
    win
}

fn ncurses_init() -> (WINDOW, WINDOW, i32, i32) {
    let mut max_x = 0;
    let mut max_y = 0;

    initscr();
    noecho();
    cbreak();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    getmaxyx(stdscr(), &mut max_y, &mut max_x);

    let message_win = create_win(max_y - 4, max_x - 1, 1, 0);
    let typing_win = create_win(3, max_x, max_y - 3, 0);

    (message_win, typing_win, max_x, max_y)
}

fn draw_screen(cs: &mut ClientState) {
    let box_message_win = create_win(cs.max_y - 4, cs.max_x, 1, 0);
    wclear(cs.message_win);
    wclear(cs.typing_win);
    mvwprintw(cs.typing_win, 1, 1, "Message: ");
    mvwprintw(stdscr(), 0, 0, "Logged in as: ");
    mvwprintw(stdscr(), 0, 14, cs.username);
    box_(box_message_win, 0, 0);
    box_(cs.typing_win, 0, 0);
    wrefresh(stdscr());
    wrefresh(cs.message_win);
    wrefresh(box_message_win);
    wrefresh(cs.typing_win);
    display_messages(cs);
}

fn remch(typing_win: WINDOW, cursor: &mut i32) {
    wmove(typing_win, 1, *cursor + 10);
    waddch(typing_win, ' ' as u32);
    wmove(typing_win, 1, *cursor + 10);
    wrefresh(typing_win);
}

fn get_str(cs: &ClientState) -> String {
    let mut buf = Buffer::new();

    // Moves cursor to the text area
    wmove(cs.typing_win, 1, 10);

    curs_set(CURSOR_VISIBILITY::CURSOR_VISIBLE);

    wrefresh(cs.typing_win);

    loop {
        let key = getch();
        match key {
            KEY_ESC => {
                break;
            }
            KEY_ENTER if buf.line.is_empty() => continue,
            KEY_ENTER => {
                curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
                return buf.line;
            }
            KEY_BACKSPACE => {
                buf.line.pop();
                buf.cursor -= 1;
                if buf.cursor < 0 {
                    buf.cursor = 0;
                }
                remch(cs.typing_win, &mut buf.cursor);
            }
            _ => {
                // keep within text area
                if buf.cursor < (cs.max_x - 12) {
                    buf.line.push(key as u8 as char);
                    waddch(cs.typing_win, key as u32);
                    wrefresh(cs.typing_win);
                    buf.cursor += 1;
                }
            }
        }
    }
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    return "".to_string();
}

fn construct_msg(raw_message: &str, username: &str) -> String {
    let mut user_message = String::new();
    user_message.push_str(username);
    user_message.push_str(": ");
    user_message.push_str(raw_message);
    user_message.push_str("\n");
    user_message
}

fn insert_mode(cs: &mut ClientState, socket: &mut TcpStream) -> bool {
    let raw_message = get_str(cs);

    if raw_message == "".to_string() {
        return false;
    } else if raw_message == "$_GET_FILE".to_string() {
        socket.write(raw_message.as_bytes()).expect("F");
        // get_message_file(socket);
        return false;
    } else {
        // create user message and send it over socket
        let user_message = construct_msg(raw_message.as_str(), cs.username);
        socket
            .write(user_message.as_bytes())
            .expect("failed to write");
        return false;
    }
}

fn display_messages(cs: &mut ClientState) {
    let messages = lines_from_file("messages2.txt");
    let mut i = 0;
    let mut msg_loc = 1;
    let space = cs.max_y - 6;
    let display_num: i32 = messages.len() as i32 - space;
    let length: i32 = messages.len() as i32;

    for message in messages {
        // This makes sure the offset is never too big or small,
        // keeps the message window constantly full of text,
        // if there is enough text to fill it that is
        if cs.offset < 0 {
            cs.offset = 0
        } else if cs.offset > length - (cs.max_y - 6) {
            cs.offset = length - (cs.max_y - 6);
        }

        if i >= (display_num - cs.offset) {
            mvwprintw(cs.message_win, msg_loc, 1, &message);
            msg_loc += 1;
            if msg_loc > space {
                msg_loc = space + cs.offset + 1;
            }
        } else if display_num < 0 {
            mvwprintw(cs.message_win, msg_loc, 1, &message);
            msg_loc += 1;
        }

        i += 1;
    }
    wrefresh(cs.message_win);
}

fn lines_from_file(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("no such file");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

#[allow(unused_assignments)]
fn normal_mode(offset: &mut i32) -> bool {
    match getch() {
        KEY_I => {
            return true;
        }
        KEY_J => {
            *offset -= 1;
            return false;
        }
        KEY_K => {
            *offset += 1;
            return false;
        }
        KEY_Q => {
            endwin();
            std::process::exit(0);
        }
        _ => return false,
    }
}
fn rem_n_char(value: &str, n: usize) -> &str {
    let mut chars = value.chars();
    for _ in 0..n {
        chars.next();
        // chars.next_back();
    }
    chars.as_str()
}

#[allow(unused_assignments)]
fn get_message_file(ip: &str) {
    loop {
        let mut first = true;
        let mut bytes_read = 0;
        let mut bytes_read_total = 0;
        let mut bytes_needed = 10000;
        // let mut last_read = 0;
        let mut socket = TcpStream::connect(ip).expect("Failed to connect");
        socket
            .write("$_GET_FILE".as_bytes())
            .expect("FAILED TO WRITE");
        let mut fd = OpenOptions::new()
            .write(true)
            // .append(true)
            .open("messages3.txt")
            .unwrap();
        while bytes_read_total < bytes_needed {
            // mvprintw(30, 30, &bytes_read_total.to_string());
            // getch();
            let mut data = [0 as u8; 500];
            bytes_read = socket.read(&mut data).expect("Failed to read");
            bytes_read_total += bytes_read;
            if first && bytes_read != 0 {
                let buf = String::from_utf8(data.to_vec()).expect("Failed to convert to string");
                let buf_split: Vec<&str> = buf.split(" ").collect();
                bytes_needed = buf_split[0].parse::<usize>().expect("Valid Int");

                first = false;
            }
            fd.write_all(&data[0..bytes_read]).expect("failed to write");
        }
        drop(fd);
        let mut fd = OpenOptions::new()
            .write(true)
            // .append(true)
            .open("messages2.txt")
            .unwrap();
        let lines = lines_from_file("messages3.txt");
        for line in lines {
            let new_line = rem_n_char(&line, 4);
            write!(fd, "{}\n", new_line).expect("failed to write");
        }
    }
}

// fn refresh_win(win: i32) {
//     loop {
//         thread::sleep(time::Duration::from_millis(50));
//         wrefresh(win as *mut i8);
//     }
// }

fn main() {
    let args: Vec<String> = env::args().collect();
    let username = &args[1];
    let ip = &args[2];

    let ip_clone = ip.clone();

    let mut typing = false;

    let mut socket: TcpStream;

    let (message_win, typing_win, max_x, max_y) = ncurses_init();

    socket = TcpStream::connect(ip).expect("Failed to connect");

    let mut client_state = ClientState {
        message_win,
        typing_win,
        offset: 0,
        max_x,
        max_y,
        username,
        // ip,
        // socket: &socket,
    };

    thread::spawn(move || get_message_file(&ip_clone));

    loop {
        draw_screen(&mut client_state);

        if typing {
            typing = insert_mode(&mut client_state, &mut socket);
        } else {
            typing = normal_mode(&mut client_state.offset);
        }
    }
}
