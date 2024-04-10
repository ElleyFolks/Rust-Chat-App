#[macro_use] extern crate rocket;

use rocket::{fs::{relative, FileServer}, response::stream::{Event, EventStream}, serde::{Deserialize, Serialize}, tokio::sync::broadcast::{channel, error::RecvError, Sender}, Shutdown};
use rocket::form::Form;
use rocket::State;
use rocket::tokio::select;

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]

struct Message {
    #[field(validate = len(..30))]
    pub room: String,
    #[field(validate = len(..20))]
    pub username: String,
    pub message: String,
}

// Sends message
#[post("/message", data = "<form>")]
fn post(form: Form<Message>, queue: &State<Sender<Message>>) {
    let _res = queue.send(form.into_inner());
}

// Receives message
#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![]{
    let mut rx = queue.subscribe();

    // Infinite loop for receiving messages
    EventStream!{
        loop{
            let msg = select! {
                // Wating for message, then match it to either Ok or Err
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                // Waiting for shutdown future to resolve
                _= &mut end => break,
            };

            // Yields a new server event
            yield Event :: json(&msg);
        }
    }
}

// Launching the rocket
#[launch]
fn rocket() -> _ {
    rocket::build()

        .manage(channel::<Message>(1024).0)
        .mount("/", routes![post, events])

        // Handler that serves static files for UI
        .mount("/", FileServer::from(relative!("static")))
}
