use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};

async fn handle_request(
    req: Request<Body>,
    visit_count: Arc<Mutex<u64>>,
) -> Result<Response<Body>, Infallible> {
    let mut count = visit_count.lock().unwrap(); // Lock the Mutex to access the count
    if req.uri().path() == "/count" {
        *count += 1; // Increment the counter
        Ok(Response::new(Body::from(format!("Visit count: {}", count))))
    } else {
        Ok(Response::new(Body::from(
            "Welcome to the Rust-powered web server!",
        )))
    }
}

#[tokio::main]
async fn main() {
    let visit_count = Arc::new(Mutex::new(0u64)); // Shared state for the visit counter

    let addr = ([127, 0, 0, 1], 8080).into(); // Bind to localhost on port 8080
    println!("Server binding to: {:?}", addr);

    let make_svc = make_service_fn(|_conn| {
        //creates a service for each incoming connection
        let visit_count = Arc::clone(&visit_count); // Clone the Arc to share the visit count with the handler
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req, Arc::clone(&visit_count))
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("The server is currently listening on http:///127.0.0.1:8080");

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}

//curl  http://127.0.0.1:8080
//curl -w '\n' http://127.0.0.1:8080
//you will need to run `sudo htop`
//oha -n 1000000 "http://127.0.0.1:8080/count"
//oha -n 100000 "http://127.0.0.1:8080/count"

//multi threaded
// Total:	6.2543 secs
// Total:	73.6288 secs
//
// not multi threaded
//Total:	6.5278 secs
//Total:	78.6375 secs
