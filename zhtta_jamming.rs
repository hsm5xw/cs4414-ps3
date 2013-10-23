//
// zhtta.rs
//
// Running on Rust 0.8
//
// Starting code for PS3
//
// Note: it would be very unwise to run this server on a machine that is
// on the Internet and contains any sensitive files!
//
// University of Virginia - cs4414 Fall 2013
// Weilin Xu and David Evans
// Version 0.2

extern mod extra;

use std::rt::io::*;
use std::rt::io::net::ip::SocketAddr;
use std::io::println;
use std::cell::Cell;
use std::{os, str, io};
use extra::arc;
use std::comm::*;

use std::rt::io::net::tcp::TcpStream;

static PORT: int = 4414;
static IP: &'static str = "0.0.0.0"; 


struct sched_msg {
    stream: Option<std::rt::io::net::tcp::TcpStream>,
    filepath: ~std::path::PosixPath
}


fn main() {
 
    let visitor_count: uint = 0; // @@@@@@@ 

    let req_vec: ~[sched_msg] = ~[];
    let shared_req_vec = arc::RWArc::new(req_vec);
    let add_vec = shared_req_vec.clone();
    let take_vec = shared_req_vec.clone();
    
    let (port, chan) = stream();
    let chan = SharedChan::new(chan);
    

    /* IMPORTANT NOTE: 	Weilin's new code for this adding file requests ('do spawn' part) is buggy. 
			So don't change the code right below ! 
    */

    // add file requests into queue.
    do spawn {
        while(true) {
            do add_vec.write |vec| {
                let tf:sched_msg = port.recv();
                (*vec).push(tf);
                println("add to queue");
            }
        }
    }
    
    // take file requests from queue, and send a response.
    // FIFO
    do spawn {
        loop {
            do take_vec.write |vec| {
                if ((*vec).len() > 0) {
                    // FILO didn't make sense in service scheduling, so we modify it as FIFO by using shift_opt() rather than pop().
                    let tf_opt: Option<sched_msg> = (*vec).shift_opt();
                    let mut tf = tf_opt.unwrap();
                    println(fmt!("shift from queue, size: %ud", (*vec).len()));

                    match io::read_whole_file(tf.filepath) { // killed if file size is larger than memory size.
                        Ok(file_data) => {
                            println(fmt!("begin serving file [%?]", tf.filepath));
                            // A web server should always reply a HTTP header for any legal HTTP request.
                            tf.stream.write("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream; charset=UTF-8\r\n\r\n".as_bytes());
                            tf.stream.write(file_data);
                            println(fmt!("finish file [%?]", tf.filepath));
                        }
                        Err(err) => {
                            println(err);
                        }
                    }
                }
            }
        }
    }

    let ip = match FromStr::from_str(IP) { Some(ip) => ip,
					   None     => {println(fmt!("Error: Invalid IP address <%s>", IP));
						        return;}
 					 };
    
    let socket = net::tcp::TcpListener::bind(SocketAddr {ip: ip, port: PORT as u16}); 

    println(fmt!("Listening on tcp port %d ...", PORT));
    let mut acceptor = socket.listen().unwrap();

    let shared_count = arc::RWArc::new( visitor_count ); // @@@@@@  


    // we can limit the incoming connection count.
    //for stream in acceptor.incoming().take(10 as uint) {

    for stream in acceptor.incoming() {
	let mut stream = stream; // @@@@@@ 
	
	match stream {
		Some(ref mut s) => 
		{
			match s.peer_name() 
			{	Some(pn) => { println( fmt!("\nPeer address: %s", pn.to_str() ));},					
				None 	 => ()
			}
		}
		None => ()
	} 			// @@@@@

	let stream = Cell::new(stream);

	let (count_port, count_chan): (Port< extra::arc::RWArc<uint> >, Chan< extra::arc::RWArc<uint> >) = std::comm::stream(); // @@@@@@   
        let count_chan = SharedChan::new(count_chan); // @@@@@@ 
        count_chan.send(shared_count.clone() ); // @@@@@@

        // Start a new task to handle the connection
        let child_chan = chan.clone();

        do spawn {
       
	    let shared_count_copy = count_port.recv(); // @@@@@@

	    /* Get Write Access */	    	    
	    do shared_count_copy.write |count|{        
			*count = *count + 1;
			println( fmt!("\ncount: %? \n", *count as int) );			
    	    }
	   	        	        	    
            let mut stream = stream.take();


            let mut buf = [0, ..500];
            stream.read(buf);
            let request_str = str::from_utf8(buf);
            
            let req_group : ~[&str]= request_str.splitn_iter(' ', 3).collect();
            if req_group.len() > 2 {
                let path = req_group[1];
                println(fmt!("Request for path: \n%?", path));
                
                let file_path = ~os::getcwd().push(path.replace("/../", ""));

                if !os::path_exists(file_path) || os::path_is_dir(file_path)
		{
                    println(fmt!("Request received:\n%s", request_str));

		    /* Get Read Access */
		    do shared_count_copy.read |count|
		    {			
   			let response: ~str = fmt!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n
			<doctype !html><html>
			<head><title>Hello, Rust!</title>
			<style> body { background-color: #111; color: #FFEEAA }
				h1 { font-size:2cm; text-align: center; color: black; text-shadow: 0 0 4mm red}
				h2 { font-size:2cm; text-align: center; color: black; text-shadow: 0 0 4mm green}
			</style>
			</head>
			<body>
				<h1>Greetings, Krusty!</h1>
				<h2>Visitor count: %u</h2>
			</body></html>\r\n", *count);	

			stream.write(response.as_bytes());	
    	    	    }

                }
                else {
                    // may do scheduling here

                    let msg: sched_msg = sched_msg{stream: stream, filepath: file_path.clone()};
                    child_chan.send(msg);
                    
                    println(fmt!("get file request: %?", file_path));
                }
            }
            println!("connection terminates\n")
        }
    }
}


