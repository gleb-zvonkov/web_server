use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    println!("Received a request: {:?}", req);
    Ok(Response::new(Body::from(
        "Welcome to the Rust-powered web server!",
    )))
}

#[tokio::main]
async fn main() {
    let addr = ([0, 0, 0, 0], 8080).into(); // Bind to all available interfaces (IPv4 + IPv6)
    println!("Server binding to: {:?}", addr);

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let server = Server::bind(&addr).serve(make_svc);

    println!("The server is currently listening on http://localhost:8080");

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}

//use hyper::server::Server;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

async fn handle_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    println!("Received a request");
    Ok(Response::new(Body::from(
        "Welcome to the Rust-powered web server!",
    )))
}

#[tokio::main]
async fn main() {
    let addr_ipv4: SocketAddr = ([0, 0, 0, 0], 8080).into(); // let addr = ([127, 0, 0, 1], 8080).into();
                                                             //server might be binding to 127.0.0.1 (IPv4), but localhost is defaulting to ::1 (IPv6)
    let addr_ipv6: SocketAddr = ([0, 0, 0, 0, 0, 0, 0, 1], 8080).into();

    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

    let server_ipv4 = Server::bind(&addr_ipv4).serve(make_svc.clone());
    let server_ipv6 = Server::bind(&addr_ipv6).serve(make_svc);

    println!("The server is currently listening on http://localhost:8080");

    let _ = tokio::try_join!(server_ipv4, server_ipv6);
}

//The server is binding to 127.0.0.1 (IPv4)
//But localhost is defaulting to ::1 (IPv6)

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tokio::runtime::Builder; // Import for custom runtime configuration

async fn handle_request(
    req: Request<Body>,
    visit_count: Arc<Mutex<u64>>,
) -> Result<Response<Body>, Infallible> {
    let mut count = visit_count.lock().unwrap(); // Lock the mutex to access and modify the count
    *count += 1; // Increment the visit count
    let response_body = format!("Visit count: {}", *count);
    Ok(Response::new(Body::from(response_body)))
}

fn main() {
    // Configure a custom runtime with multiple worker threads to take advantage of multiple CPU cores
    let rt = Builder::new_multi_thread()
        .worker_threads(num_cpus::get()) // Automatically sets worker threads equal to CPU core count
        .enable_all() // Enables all necessary Tokio components (like timers, IO, etc.)
        .build()
        .unwrap();

    let visit_count = Arc::new(Mutex::new(0)); // Shared, thread-safe counter

    // Run the server within the custom multi-threaded runtime
    rt.block_on(async {
        let addr = ([127, 0, 0, 1], 8080).into();

        let make_svc = make_service_fn(|_conn| {
            let visit_count = Arc::clone(&visit_count); // Clone the Arc to share the visit count with each handler
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    handle_request(req, Arc::clone(&visit_count))
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        println!("The server is running on http://localhost:8080");

        // Handle server errors if any occur
        if let Err(e) = server.await {
            eprintln!("Server error: {}", e);
        }
    });
}
