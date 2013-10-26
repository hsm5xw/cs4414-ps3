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
// Version 0.3

extern mod extra;

use std::rt::io::*;
use std::rt::io::net::ip::SocketAddr;
use std::io::println;
use std::cell::Cell;
use std::{os, str, io};
use extra::arc;
use extra::time;
use std::comm::*;
use std::hashmap::HashMap;

use std::rt::io::net::tcp::TcpStream;
use std::cmp::Ord;
use std::num::log2;

use extra::priority_queue::*;

static PORT: int = 4414;
static IP: &'static str = "127.0.0.1";


struct sched_msg {
    stream: Option<std::rt::io::net::tcp::TcpStream>,
    filepath: ~std::path::PosixPath,
    priority: int  // @@@@@@@@@@@@@@@
}

impl std::cmp::Ord for sched_msg
{
	fn lt(&self, other: &sched_msg) -> bool { (*self).priority < (*other).priority }
	fn le(&self, other: &sched_msg) -> bool { (*self).priority <= (*other).priority }
	fn ge(&self, other: &sched_msg) -> bool { (*self).priority >= (*other).priority }
	fn gt(&self, other: &sched_msg) -> bool { (*self).priority > (*other).priority } 
}


// determine if the request is from a Charlottesville client

fn is_Wahoo_Client(peer_addr: ~str) ->bool
{
        let WahooIPs: ~[~str] = ~[~"128.143.", ~"137.54."]; // probably add some more ip addresses on the way
        let mut isWahoo = false;
        let mut index = 0;

        while index < WahooIPs.len()
        {
                if peer_addr.starts_with(WahooIPs[index])
                {
                        isWahoo = true;
                        break;
                }
                index += 1;
        }
        //println( fmt!("is wahoo? : %?\n", isWahoo) );
        return isWahoo;
}

// get the size of a file

fn get_fileSize(file_path: &Path) -> int
{
        let mut filestream = std::rt::io::file::open(file_path, Open, Read);
        let mut size:int = 0;

        // find the File size
        match filestream{
                Some(ref mut reader) =>
                {                                                                        
                        reader.seek(0,SeekEnd);
                        size = reader.tell() as int;
                        reader.seek(0, SeekSet); // rewind the file back

                        println(fmt!("\nfile size: %? bytes \n", size ));
                },
                None => { fail!("\nCannot read the storage file \n");},
        }
        return size;        
}


fn main() {
 
    let visitor_count: uint = 0; // @@@@@@@

/*
    let req_vec: ~[sched_msg] = ~[];
    let shared_req_vec = arc::RWArc::new(req_vec);
    let add_vec = shared_req_vec.clone();
    let take_vec = shared_req_vec.clone();
*/

    /* replace data structure to priority queue (this is not the only region changed though) */

    let req_pq: PriorityQueue<sched_msg> = PriorityQueue::new();
    let shared_req_pq = arc::RWArc::new(req_pq);
    let add_pq = shared_req_pq.clone();
    let take_pq = shared_req_pq.clone();
   

    let (port, chan) = stream();
    let chan = SharedChan::new(chan);
    

    // dequeue file requests, and send responses.
    // FIFO
    do spawn {
        	let (sm_port, sm_chan) = stream();
        	let (sm_port2, sm_chan2) = stream();

        	// a task for sending responses.
        	do spawn {
         		let mut cache: HashMap<~str, ~[u8]> = std::hashmap::HashMap::new();
         		let mut cacheTimes: HashMap<~str, ~i64> = std::hashmap::HashMap::new();
			let mut fileAccessTimes: HashMap<~str, ~i64> = std::hashmap::HashMap::new();
	   		let maxCacheSize = 2;
	    		let maxCacheFileSize = 500000;
            		loop {
                		sm_chan2.send(1);
               			let mut tf: sched_msg = sm_port.recv(); // wait for the dequeued request to handle
                
                		let modifiedTime = match file::stat(tf.filepath)
				{
                        		Some(s) => {s.modified}
                        		None 	=> {-1} 
				};

                		if cache.contains_key_equiv(&tf.filepath.to_str()) &&
                           		*cacheTimes.get_copy(&tf.filepath.to_str()) as u64 > modifiedTime 
				{	
					println(fmt!("begin serving cached file [%?]", tf.filepath));
                        		// A web server should always reply a HTTP header for any legal HTTP request.
                        		tf.stream.write("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n".as_bytes());
                        		let data = cache.get_copy(&tf.filepath.to_str());
                        		tf.stream.write(data);
					fileAccessTimes.insert(tf.filepath.to_str(), ~time::get_time().sec);
                        		println(fmt!("finish file [%?]", tf.filepath));
                		}
                		else {
                 			let file = io::read_whole_file(tf.filepath);

            	 			match file { // killed if file size is larger than memory size.

                         			Ok(file_data) => 
						{
                          				println(fmt!("begin serving file [%?]", tf.filepath));
                          				// A web server should always reply a HTTP header for any legal HTTP request.
                          				tf.stream.write("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n".as_bytes());
                                			tf.stream.write(file_data);
                                			println(fmt!("finish file [%?]", tf.filepath));
                                			let file_size = get_fileSize(tf.filepath);
							if file_size < maxCacheFileSize {
								cache.insert(tf.filepath.to_str(), file_data);
								cacheTimes.insert(tf.filepath.to_str(), ~time::get_time().sec);
								fileAccessTimes.insert(tf.filepath.to_str(), ~time::get_time().sec);
								if cache.len() > maxCacheSize {
									let clone = fileAccessTimes.clone();
									let mut iterator = clone.iter();
						
									let mut firstAccessTime: i64;
									let mut firstAccessKey: &~str;
									let mut option = iterator.next();
									match option {
										Some(s) => {
											let (key, value) = s;
											firstAccessTime = **value;
											firstAccessKey = key;
										},
										None => {fail!("hash map iteration failed");}
									}
									let mut i = 0;
									while i < cache.len() - 1 {
										option = iterator.next();
										match option {
											Some(s) => {
												let (key, value) = s;
												if **value < firstAccessTime {
													firstAccessTime = **value;
													firstAccessKey = key;
												}
											},
											None => {fail!("hash map iteration failed");}
										}
										i += 1;
									}
									cache.remove(firstAccessKey);
									cacheTimes.remove(firstAccessKey);
									fileAccessTimes.remove(firstAccessKey);
								}
							}
                             			}
                             			Err(err) => 
						{	println(err);
						}
                    			} 
                		} // else
           		} // loop
        	} // spawn (inner)
        
        loop {
	    sm_port2.recv();
            port.recv(); // wait for arrving notification
            do take_pq.write |priority_Q| {
                if ((*priority_Q).len() > 0) {
                    // LIFO didn't make sense in service scheduling, so we modify it as FIFO by using shift_opt() rather than pop().
                    
		    //let tf_opt: Option<sched_msg> = (*priority_Q).pop();
                    let tf = (*priority_Q).pop();
                    println(fmt!("shift from queue, size: %ud", (*priority_Q).len()));    // ^^^^^^^^^^^^^^^^^^^ probably need to change it later
                    sm_chan.send(tf); // send the request to send-response-task to serve.
                }
            }
        }
    } // spawn (outer)


    let ip = match FromStr::from_str(IP) { Some(ip) => ip,
                                           None     => {println(fmt!("Error: Invalid IP address <%s>", IP));
                                                    	return;}
                                         };
    

    let socket = net::tcp::TcpListener::bind(SocketAddr {ip: ip, port: PORT as u16});
    
    println(fmt!("Listening on %s:%d ...", ip.to_str(), PORT));
    let mut acceptor = socket.listen().unwrap();


    let shared_count = arc::RWArc::new( visitor_count ); // @@@@@@


    for stream in acceptor.incoming() {
        let mut stream = stream; // @@@@@@
        let mut peer_addr: ~str = ~"";        

        match stream {
                Some(ref mut s) =>
                {
                        match s.peer_name() {        
                                Some(pn) =>
                                {   peer_addr = pn.to_str();
                                    println( fmt!("\nPeer address: %s", peer_addr ));                                        
                                },                                        
                                None     => ()
                        }
                }
                None => ()
        }                         

        let isWahoo: bool = is_Wahoo_Client(peer_addr);
        println( fmt!("is wahoo? : %?\n", isWahoo) );

        let stream = Cell::new(stream);

        let (count_port, count_chan): (Port< extra::arc::RWArc<uint> >, Chan< extra::arc::RWArc<uint> >) = std::comm::stream(); // @@@@@@
        let count_chan = SharedChan::new(count_chan); // @@@@@@
        count_chan.send(shared_count.clone() ); // @@@@@@

        // Start a new task to handle the connection
        let child_chan = chan.clone();
        let child_add_pq = add_pq.clone();

        do spawn {
       
         let shared_count_copy = count_port.recv(); // @@@@@@

         /* Get Write Access */                 
         do shared_count_copy.write |count|{
                        *count = *count + 1;
                        println( fmt!("count: %? \n", *count as int) );                        
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
                    // Requests scheduling

                    let file_size:int = get_fileSize(file_path); // @@@@@@@@@@@

		    // Rust 0.8 only provides a maximum priority queue
		    let mut adjusted_priority: int = (-1) * (log2( file_size as float) as int); // multiply priority by (-1) to use it as minimum priority queue

		    // Implement 'Wahoo-First' scheduling
		    if(isWahoo) { adjusted_priority += 3;}

                    let msg: sched_msg = sched_msg{stream: stream, filepath: file_path.clone(), priority: adjusted_priority }; 

                    let (sm_port, sm_chan) = std::comm::stream();
                    sm_chan.send(msg);
                    
		    // Implement 'Shortest-Processing-Time-First' scheduling
                    do child_add_pq.write |priority_Q| {
                        let msg = sm_port.recv();
                        (*priority_Q).push(msg); // enqueue new request.
                        println("add to queue");
                    }
                    child_chan.send(""); //notify the new arriving request.
                    
                    println(fmt!("get file request: %?", file_path));
                }
            }
            println!("connection terminates\n")
        }
    }
}



