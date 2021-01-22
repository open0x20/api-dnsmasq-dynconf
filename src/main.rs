use std::io::prelude::*;
use std::process::Command;
use std::fs::File;
use actix_web::{web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use nix::unistd::Uid;

#[derive(Debug, Serialize, Deserialize)]
struct EntryRequestDto {
    name: String,
    ip: String,
    secret: String
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Check for root privileges first
    if !Uid::effective().is_root() {
        panic!("You must run this executable with root permissions");
    }

    println!("Starting up dnsmasq-dynconf");
    initialize_files();

    println!("Starting REST-API on 0.0.0.0:7878");
    HttpServer::new(|| {
        App::new()
            // GET /list
            .service(web::resource("/list").route(web::get().to(action_list)))
            // PUT /add
            .service(web::resource("/add").route(web::put().to(action_add)))
            // POST /DELETE
            .service(web::resource("/delete").route(web::post().to(action_delete)))
    })
    .bind("0.0.0.0:7878")?
    .run()
    .await
}

async fn action_list() -> HttpResponse {
    // Load entries from custom.conf
    let custom_entries = load_custom_dnsmasq_entries_from_file();

    // Parse all custom entries into a json representation
    let json_response = parse_address_vector_into_json_string(custom_entries);

    HttpResponse::Ok()
        .set_header("Content-Type", "application/json")
        .body(json_response)
}

async fn action_add(item: web::Json<EntryRequestDto>) -> HttpResponse {
    // Check for authorization
    if !is_authorized(&item.0.secret[..]) {
        return HttpResponse::Unauthorized().finish();
    }

    // Load entries from custom.conf
    let mut custom_entries = load_custom_dnsmasq_entries_from_file();

    // Add the requested entry to our address vector
    custom_entries.push(vec![item.0.name, item.0.ip]);

    // Write to file custom.con (overwriting)
    write_to_custom_file(parse_address_vector_into_address_string(custom_entries));

    // Reload the dnsmasqd.service
    let _ = Command::new("/usr/bin/systemctl reload dnsmasq.service").spawn();

    HttpResponse::Ok().finish()
}

async fn action_delete(item: web::Json<EntryRequestDto>) -> HttpResponse {
    // Check for authorization
    if !is_authorized(&item.0.secret[..]) {
        return HttpResponse::Unauthorized().finish();
    }

    // Load entries from custom.conf
    let mut custom_entries = load_custom_dnsmasq_entries_from_file();

    // Find entries that have to be removed
    let mut indicies_to_delete: Vec<usize> = Vec::new();
    for i in 0..custom_entries.len() {
        if (item.0.name == custom_entries[i][0]) && (item.0.ip == custom_entries[i][1]) {
            indicies_to_delete.push(i);
        }
    }

    // Remove entries
    indicies_to_delete.reverse();
    for itd in indicies_to_delete {
        custom_entries.remove(itd);
    }
    
    // Write to file custom.con (overwriting)
    write_to_custom_file(parse_address_vector_into_address_string(custom_entries));

    // Reload the dnsmasqd.service
    let _ = Command::new("/usr/bin/systemctl reload dnsmasq.service").spawn();

    HttpResponse::Ok().finish()
}

fn initialize_files() {
    println!("Checking files (should be owned by root)");
    if !File::open("/etc/dnsmasq-dynconf.token").is_ok() {
        println!("Creating empty token file at '/etc/dnsmasq-dynconf.token'...");
        if !File::create("/etc/dnsmasq-dynconf.token").is_ok() {
            panic!("Could not create '/etc/dnsmasq-dynconf.token'!")
        }
    }

    if !File::open("/etc/dnsmasq.d/custom.conf").is_ok() {
        println!("Creating empty config file at '/etc/dnsmasq.d/custom.conf'...");
        if !File::create("/etc/dnsmasq.d/custom.conf").is_ok() {
            panic!("Could not create '/etc/dnsmasq.d/custom.conf'!")
        }
    }
}

fn is_authorized(secret: &str) -> bool {
    let mut token_file = File::open("/etc/dnsmasq-dynconf.token").unwrap();
    let mut token = String::new();
    let _ = token_file.read_to_string(&mut token);

    if token.trim() != secret {
        false
    } else {
        true
    }
}

fn load_custom_dnsmasq_entries_from_file() -> Vec<Vec<String>>{
    // Load all entries from /etc/dnsmasq.d/custom.conf
    let mut custom_file_read = File::open("/etc/dnsmasq.d/custom.conf").unwrap();
    let mut contents = String::new();
    let _ = custom_file_read.read_to_string(&mut contents);

    let entries_raw: Vec<&str> = contents.split_terminator("\n").collect();
    let mut entries: Vec<Vec<String>> = Vec::new();

    for e in entries_raw {
        entries.push(parse_address_string(e));
    }

    entries
}

/* Extract the two values of a dnsmasq "address" entry string and return
 * them in a vector of length 2. It looks like this:
 *
 * assert_eq!(
 *   parse_address_string("address=/test.myhost.de/127.0.0.1"),
 *   vec!["test.myhost.de", "127.0.0.1"]
 * );
 */
fn parse_address_string(address: &str) -> Vec<String> {
    let mut parts: Vec<String> = address.split_terminator("/").map(|x| String::from(x)).collect();
    parts.remove(0);

    parts
}

/* Creates a json representation of an address list.
 *
 * let adr1 = "address=/test1/127.0.0.1";
 * let adr2 = "address=/test2/127.0.0.1";
 * let list_of_addrs = vec![
 *   parse_address_string(adr1),
 *   parse_address_string(adr2),
 * ];
 *
 * assert_eq!(
 *   parse_address_vector_into_json_string(list_of_addrs)
 *   "{\"addresses\":[{\"address\":\"test1\",\"ip\":\"127.0.0.1\"},{\"address\":\"test2\",\"ip\":\"127.0.0.1\"},]}"
 * );
 */
fn parse_address_vector_into_json_string(addresses: Vec<Vec<String>>) -> String {
    let mut json: String = String::new();
    json.push_str("{\"addresses\":[");

    for adr in addresses {
        json.push_str("{\"address\":\"");
        json.push_str(&adr[0][..]);
        json.push_str("\",\"ip\":\"");
        json.push_str(&adr[1][..]);
        json.push_str("\"},");
    }

    json.push_str("]}");

    json
}

/* Creates a dnsmasq interpreterable representation of an address list.
 *
 * let adr1 = "address=/test1/127.0.0.1"
 * let adr2 = "address=/test2/127.0.0.1"
 * let list_of_addrs = vec![
 *   parse_address_string(adr1),
 *   parse_address_string(adr2),
 * ];
 *
 * assert_eq!(
     parse_address_vector_into_address_string(list_of_addrs),
 *   "address=/test1/127.0.0.1\naddress=/test2/127.0.0.1\n"
 * );
 */
fn parse_address_vector_into_address_string(addresses: Vec<Vec<String>>) -> String {
    let mut address_string: String = String::new();

    for adr in addresses {
        address_string.push_str(&format!("address=/{}/{}\n", adr[0], adr[1])[..]);
    }

    address_string
}

/* Writes a string into the /etc/dnsmasq.d/custom.conf file. The file will
 * be created anew.
 */
fn write_to_custom_file(content: String) {
    let mut custom_file_write = File::create("/etc/dnsmasq.d/custom.conf").unwrap();
    let _ = custom_file_write.write(content.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address_string() {
        assert_eq!(
            parse_address_string("address=/test.myhost.de/127.0.0.1"),
            vec!["test.myhost.de", "127.0.0.1"]
        );
    }

    #[test]
    fn test_parse_address_vector_into_json_string() {
        let adr1 = "address=/test1/127.0.0.1";
        let adr2 = "address=/test2/127.0.0.1";
        let list_of_addrs = vec![
            parse_address_string(adr1),
            parse_address_string(adr2),
        ];

        assert_eq!(
            parse_address_vector_into_json_string(list_of_addrs),
            "{\"addresses\":[{\"address\":\"test1\",\"ip\":\"127.0.0.1\"},{\"address\":\"test2\",\"ip\":\"127.0.0.1\"},]}"
        );
    }

    #[test]
    fn test_parse_address_vector_into_address_string() {
        let adr1 = "address=/test1/127.0.0.1";
        let adr2 = "address=/test2/127.0.0.1";
        let list_of_addrs = vec![
            parse_address_string(adr1),
            parse_address_string(adr2),
        ];

        assert_eq!(
            parse_address_vector_into_address_string(list_of_addrs),
            "address=/test1/127.0.0.1\naddress=/test2/127.0.0.1\n"
        );
    }
}