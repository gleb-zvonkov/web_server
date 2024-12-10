//Gleb Zvonkov
//Dec 9, 2024
//ECE1724

use hyper::service::{make_service_fn, service_fn}; //components for creating HTTP services
use hyper::{Body, Method, Request, Response, Server, StatusCode}; //core HTTP
use serde::{Deserialize, Serialize}; //serializing and deserializing data structures
use serde_json::json; //for creating JSON objects
use std::convert::Infallible; //used for functions or closures that never fail
use std::sync::Arc; //atomic reference counting and thread-safe shared ownership
use tokio::fs as async_fs; //asynchronous file system operations
use tokio::signal::unix::{signal, SignalKind}; //handling Unix signals in an asynchronous way
use tokio::sync::{Mutex, RwLock}; //Mutex  and RwLock for a lock that supports multiple readers or a single writer

const DB_FILE: &str = "music_library.json"; //json file where we permanetley store data

#[derive(Serialize, Deserialize, Clone)] //derive implementations for Serialize, Deserialize, Clone
                                         //serialize allows it to be converted into json, deserialize is the opposite
struct Song {
    id: u64,
    title: String,
    artist: String,
    genre: String,
    play_count: u64,
}

#[derive(Deserialize)]
struct NewSong {
    //to diffirentiate when a new song is recived
    title: String,
    artist: String,
    genre: String,
}

//Function that loads in json data if it exists
async fn load_library() -> Vec<Song> {
    if let Ok(data) = async_fs::read_to_string(DB_FILE).await {
        match serde_json::from_str(&data) {
            Ok(library) => library,
            Err(_) => {
                //failed to desirialize create a new libary
                Vec::new()
            }
        }
    } else {
        //if it doesnt exists create a new vector
        Vec::new()
    }
}

//Saves songs to json
async fn save_library(library: &[Song]) {
    if let Ok(data) = serde_json::to_string(library) {
        if let Err(e) = async_fs::write(DB_FILE, data).await {
            eprintln!("Failed to save library: {}", e);
        }
    } else {
        eprintln!("Failed to serialize library.");
    }
}

//Vec<Song> is vector of songs
//Mutex<Vec<Song>> protected by mutex, so only one thread can modify or access the vector at a time
//Arc<Mutex<Vec<Song>>>  Atomic Reference Counted, allows multiple threads to share ownership of a value
type SongDb = Arc<RwLock<Vec<Song>>>; //Vector of songs wrapped in read write lock wrapped in atomic refrence count
type VisitCounter = Arc<Mutex<u64>>; //visit counter wrapped in mutex wrapped in atomic refence count

//This functions all possible requests from clients
async fn handle_request(
    req: Request<Body>,
    visit_counter: VisitCounter,
    song_db: SongDb,
) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        //match on the method and path

        //If no path just print welcome message
        (&Method::GET, "/") => Ok(Response::new(Body::from(
            "Welcome to the Rust-powered web server!",
        ))),

        //Get the count,  make sure that your solution not only uses all CPU cores concurrently, but also maintains the correct visit count
        (&Method::GET, "/count") => {
            let mut count = visit_counter.lock().await;
            *count += 1; //increment it
            let response = format!("Visit count: {}", count); //format it
            Ok(Response::new(Body::from(response))) //return the count
        }

        // Handle adding a new song
        (&Method::POST, "/songs/new") => {
            let body_bytes = hyper::body::to_bytes(req.into_body()).await.unwrap();
            let new_song: NewSong = match serde_json::from_slice(&body_bytes) {
                Ok(song) => song,
                Err(_) => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("Invalid JSON"))
                        .unwrap())
                }
            };
            let new_id = song_db.read().await.last().map(|song| song.id).unwrap_or(0) as u64 + 1; //read the id from the last record and incrment it
            let song = Song {
                id: new_id,
                title: new_song.title,
                artist: new_song.artist,
                genre: new_song.genre,
                play_count: 0,
            };
            let response_body = serde_json::to_string(&song).unwrap(); //serialize the song
            song_db.write().await.push(song); //add the new song
            Ok(Response::new(Body::from(response_body))) //return a response
        }

        // Handle searching for songs
        (&Method::GET, "/songs/search") => {
            let query = req.uri().query().unwrap_or(""); // Extract query parameters
            let mut title_query = None;
            let mut artist_query = None;
            let mut genre_query = None;
            for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
                match key.as_ref() {
                    "title" => title_query = Some(value.to_string().to_lowercase()),
                    "artist" => artist_query = Some(value.to_string().to_lowercase()),
                    "genre" => genre_query = Some(value.to_string().to_lowercase()),
                    _ => {}
                }
            }

            let db = song_db.read().await; //Read the song vector
            let filtered_songs: Vec<&Song> = db // Perform filtering to search for songs with keywords
                .iter()
                .filter(|song| {
                    // Check title, artist, and genre only if queries are provided
                    title_query
                        .as_ref()
                        .map_or(true, |title| song.title.to_lowercase().contains(title))
                        && artist_query
                            .as_ref()
                            .map_or(true, |artist| song.artist.to_lowercase().contains(artist))
                        && genre_query
                            .as_ref()
                            .map_or(true, |genre| song.genre.to_lowercase().contains(genre))
                })
                .collect();

            let response_body = serde_json::to_string(&filtered_songs).unwrap();
            Ok(Response::new(Body::from(response_body))) // Respond with the filtered songs
        }

        // Handle playing a song
        (&Method::GET, path) if path.starts_with("/songs/play/") => {
            let song_id: u64 = match path.strip_prefix("/songs/play/") {
                Some(id_str) => match id_str.parse() {
                    Ok(id) => id,
                    Err(_) => {
                        return Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from("Invalid song ID"))
                            .unwrap())
                    }
                },
                None => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("Song ID missing"))
                        .unwrap())
                }
            };

            let mut db = song_db.write().await; //we are gonna update the database

            if let Some(song) = db.iter_mut().find(|s| s.id == song_id) {
                // Find the song by ID and increment its play count
                song.play_count += 1; //increment play count
                let response_body = serde_json::to_string(&song).unwrap();
                Ok(Response::new(Body::from(response_body)))
            } else {
                let error_response = json!({ "error": "Song not found" });
                let response_body = serde_json::to_string(&error_response).unwrap();
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from(response_body))
                    .unwrap())
            }
        } //end if

        // Return 404 for other routes
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap()),
    } //end of matchs
}

#[tokio::main]
async fn main() {
    let song_db: SongDb = Arc::new(RwLock::new(load_library().await)); //song database
    let visit_counter: VisitCounter = Arc::new(Mutex::new(0)); //visitor count

    let addr = "[::]:8080".parse().unwrap(); //When you bind to [::], listens on all IPv6 interfaces. It also listens on IPv4 interfaces.

    let make_svc = make_service_fn(|_conn| {
        let song_db = Arc::clone(&song_db); // Clone the Arc to share the song_db with each handler
        let visit_counter = Arc::clone(&visit_counter);
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req, Arc::clone(&visit_counter), Arc::clone(&song_db))
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_svc); //bind the server with the service function

    println!("The server is currently listening on localhost:8080.");

    //Catch intrerupt signals crl-c and kill-15
    let shutdown_signal = async {
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to listen for INT"); //for ctrl-c
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to listen for TERM"); //for kill-15
        tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
        }
        save_library(&song_db.read().await).await;
    };
    if let Err(e) = tokio::select! {   //once the server has an error
        res = server => res,
        _ = shutdown_signal => Ok(()),
    } {
        eprintln!("Server error: {}", e);
    }
} //end main
