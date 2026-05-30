//! Send a single email.
//!
//! Run with an API key in the environment:
//!
//! ```bash
//! ANYPOST_API_KEY=ap_... cargo run --example send
//! ```

use anypost::{Client, Error, SendEmail};

#[tokio::main]
async fn main() {
    // Reads ANYPOST_API_KEY from the environment. Pass the key explicitly with
    // Client::new("ap_...") if you prefer.
    let client = match Client::from_env() {
        Ok(client) => client,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let message = SendEmail::new("Acme <you@yourdomain.com>", ["someone@example.com"])
        .subject("Hello from Anypost")
        .html("<p>It worked.</p>");

    match client.email.send(&message).await {
        Ok(email) => println!("Queued {}", email["id"]),
        Err(Error::Validation(e)) => {
            eprintln!("Validation failed: {:?}", e.errors);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Send failed: {e}");
            std::process::exit(1);
        }
    }
}
