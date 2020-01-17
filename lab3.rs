extern crate threadpool;
extern crate rand;

use threadpool::ThreadPool;
use rand::Rng;
use std::char;
use std::io::{Read, Write};
use std::str::from_utf8;
use std::net::{TcpListener, TcpStream, Shutdown};
use std::env;
use std::io;

fn get_session_key() -> String {
    let mut result: String = "".to_string();
    for _ in 1..11 {
        result += &format!("{}", rand::thread_rng().gen_range(0.0, 9.0) as u64 + 1);
    }
    return result;
}

fn get_hash_str() -> String {
    let mut li: String = "".to_string();
    for _ in 1..6 {
        li += &format!("{}", (rand::thread_rng().gen_range(0.0, 6.0) as u64 + 1) as u64);
    }
    return li;
}

fn next_session_key(hash: &str, session_key: &str) -> String {
    if hash == "" {
        panic!("Hash code is empty");
    }
    for ch in hash.chars() {
        if !ch.is_digit(10) {
            panic!("Hash code contains non-digit letter {}", ch);
        }
    }
    let mut result = 0;
    for ch in hash.chars() {
        result += calc_hash(session_key.to_string(), ch.to_string().parse::<u64>().unwrap()).parse::<u64>().unwrap();
    }
    let mut ret = String::new(); 
    if result.to_string().len() < 10 {
        ret = "0000000000".to_string() + &result.to_string()[0..result.to_string().len()].to_string();
    } else {
        ret = "0000000000".to_string() + &result.to_string()[0..10].to_string();
    }
    return ret[ret.len() - 10..ret.len()].to_string();
}

fn calc_hash(session_key: String, val: u64) -> String{
    let mut result: String = "".to_string();
    if val == 1 {
        result = "00".to_string() + &(session_key[0..5].parse::<u64>().unwrap() % 97).to_string();
        return result[result.len() - 2..result.len()].to_string()
    }
    if val == 2 {
        for i in 0..session_key.len() {
            result += &session_key.chars().nth(session_key.len() - i - 1).unwrap().to_string();
        }
        return result;
    }
    if val == 3 {
        return session_key[session_key.len() - 5..session_key.len()].to_string() + &session_key[0..5].to_string();
    }
    if val == 4 {
        let mut num = 0;
        for i in 1..9{
            num += session_key.chars().nth(i).unwrap().to_string().parse::<u64>().unwrap() + 41;
        }
        return num.to_string();
    }
    if val == 5 {
        let mut ch: char;
        let mut num = 0;
        for i in 0..session_key.len() {
            ch = ((session_key.chars().nth(i).unwrap() as u8) ^ 43) as char;
            if !ch.is_digit(10) {
                ch = (ch as u8) as char;
            }
            num += ch as u64;
        }
        return num.to_string();
    }
    return (session_key.parse::<u64>().unwrap() + val).to_string();
}

fn client(ip: &str) {
    match TcpStream::connect(ip.to_string()) {
        Ok(mut stream) => {
            println!("Successful connection");

            let mut data = [0 as u8; 60];
            let hash_str = get_hash_str();
            let mut session_key = get_session_key();
            println!("{} {}\n\nWaiting for answer...", &hash_str, &session_key);
            stream.write(&(hash_str.clone() + &session_key).into_bytes()).unwrap();
            match stream.read(&mut data) {
                Ok(_) => {
                    let mut received_key = from_utf8(&data).unwrap()[0..10].to_string();
                    let mut next_key = next_session_key(&hash_str, &session_key);
                    println!("Received key: {}\nGenerated key: {}\n", received_key, next_key);
                    if received_key == next_key {
                        loop {
                            session_key = next_session_key(&hash_str, &next_key);
                            next_key = next_session_key(&hash_str, &session_key);

                            println!("Your message: ");
                            let mut message: String = "".to_string();

                            //io::stdin().read_line(&mut message);
                            println!("\nSent key: {}\n\nWaiting for answer...", &session_key);
                            stream.write(&(session_key + &message).into_bytes()).unwrap();

                            match stream.read(&mut data) {
                                Ok(_) => {
                                    received_key = from_utf8(&data).unwrap()[0..10].to_string();
                                    let response = from_utf8(&data).unwrap()[10..60].to_string();
                                    println!("Message from server: {}", response);
                                    println!("Received key: {}\nGenerated key: {}\n", received_key, next_key);
                                    if received_key != next_key { 
                                        println!("Keys are different. Stopping connection..."); 
                                        break 
                                    }
                                }, 
                                Err(e) => {
                                    println!("Failed to receive data: {}", e);
                                }
                            }
                        }
                    } else { println!("Keys are different. Stopping connection..."); }    
                }, 
                Err(e) => {
                    println!("Failed to connect: {}", e);
                }
            }
        },
        Err(e) => {
            println!("Failed to connect: {}", e);
        }
    }
    println!("Client DESTROYED");
}

fn run_connection(mut stream: TcpStream) {
    let mut hash_message = [0 as u8; 15]; 
    let mut message = [0 as u8; 60];
    match stream.read(&mut hash_message) {
        Ok(_) => {
            let session_hash = from_utf8(&hash_message).unwrap()[0..5].to_string();
            let mut received_key = from_utf8(&hash_message).unwrap()[5..15].to_string();
            println!("{} {}", &session_hash, &received_key);
            let mut new_key = next_session_key(&session_hash,&received_key);
            let mut result = new_key.clone().into_bytes();
            stream.write(&result).unwrap();
            loop { match stream.read(&mut message) {
                Ok(_) => {
                    received_key = from_utf8(&message).unwrap()[0..10].to_string();
                    let txt = from_utf8(&message).unwrap()[10..60].to_string();
                    new_key = next_session_key(&session_hash, &received_key);
                    println!("Message from client {} Received: {}\nReceived key: {}\nGenerated key: {}\n", stream.peer_addr().unwrap(), &txt, &received_key, &new_key);
                    result = (new_key + &txt.to_uppercase()).clone().into_bytes();
                    stream.write(&result).unwrap();
                },
                Err(e) => {
                    println!("Connection error with {}\nError: {}", stream.peer_addr().unwrap(), e);
                    stream.shutdown(Shutdown::Both).unwrap();
                    break
                }
            } }
        },
        Err(e) => {
            println!("Connection error with {}\nError: {}", stream.peer_addr().unwrap(), e);
            stream.shutdown(Shutdown::Both).unwrap();
        }
    }
}

fn server(port: &str, n: usize) {
    let pool = ThreadPool::new(n);
    let listener = TcpListener::bind("0.0.0.0".to_string() + &port.to_string()).unwrap();
    println!("Server is listening");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
                pool.execute(move || {
                    run_connection(stream)
                });
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    drop(listener);
}

fn main() {
	let args: Vec<String> = env::args().collect();
    
    if (args.len() == 3) && (args[1] == "ip:port") {
            client(&args[2]);
    } else {
        if (args.len() == 5) && (args[1] == "port")  && (args[3] == "-n") {
            server(&args[2], args[4].parse::<usize>().unwrap());
        }
    }
}