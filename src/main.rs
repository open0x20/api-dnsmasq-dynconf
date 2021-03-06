use std::io::prelude::*;
use std::process::Command;
use std::fs::File;
use actix_web::{web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use nix::unistd::Uid;
use daemonize::Daemonize;

#[derive(Debug, Serialize, Deserialize)]
struct EntryRequestDto {
    name: String,
    ip: String,
    secret: String
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let stdout = File::create("/tmp/dnsmdcd.out").unwrap();
    let stderr = File::create("/tmp/dnsmdcd.err").unwrap();

    let daemonize = Daemonize::new()
        .pid_file("/run/dnsmdcd.pid")
        .working_directory("/tmp")
        .user("root")
        .group("root")
        .stdout(stdout)
        .stderr(stderr);

    match daemonize.start() {
        Ok(_) => {
            // Check for root privileges first
            if !Uid::effective().is_root() {
                panic!("You must run this executable with root permissions");
            }

            println!("Starting up dnsmdcd");
            initialize_files();

            println!("Starting dnsmasq dynamic configurator daemon (dnsmdcd) on 127.0.0.1:47078");
            HttpServer::new(|| {
                App::new()
                    // GET /list
                    .service(web::resource("/list").route(web::get().to(action_list)))
                    // PUT /add
                    .service(web::resource("/add").route(web::put().to(action_add)))
                    // POST /delete
                    .service(web::resource("/delete").route(web::post().to(action_delete)))
            })
            .bind("127.0.0.1:47078")?
            .run()
            .await
        },
        Err(e) => {
            eprintln!("Error, {}", e);
            Ok(())
        }
    }
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
    let custom_entries_original = custom_entries.clone();

    // Add the requested entry to our address vector
    custom_entries.push(vec![item.0.name, item.0.ip]);

    // Write to file custom.con (overwriting)
    write_to_custom_file(parse_address_vector_into_address_string(custom_entries));

    // Try restarting the dnsmasqd.service, rollback in case of error
    if !restart_dnsmasq() {
        // Rollback
        write_to_custom_file(parse_address_vector_into_address_string(custom_entries_original));
        // Restart again with old configuration file
        restart_dnsmasq();

        HttpResponse::Conflict().finish()
    } else {
        HttpResponse::Ok().finish()
    }
}

async fn action_delete(item: web::Json<EntryRequestDto>) -> HttpResponse {
    // Check for authorization
    if !is_authorized(&item.0.secret[..]) {
        return HttpResponse::Unauthorized().finish();
    }

    // Load entries from custom.conf
    let mut custom_entries = load_custom_dnsmasq_entries_from_file();
    let custom_entries_original = custom_entries.clone();

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

    // Try restarting the dnsmasqd.service, rollback in case of error
    if !restart_dnsmasq() {
        // Rollback
        write_to_custom_file(parse_address_vector_into_address_string(custom_entries_original));
        // Restart again with old configuration file
        restart_dnsmasq();

        HttpResponse::Conflict().finish()
    } else {
        HttpResponse::Ok().finish()
    }
}

fn initialize_files() {
    println!("Checking files (should be owned by root)");
    if !File::open("/etc/dnsmdcd.token").is_ok() {
        println!("Creating empty token file at '/etc/dnsmdcd.token'...");
        if !File::create("/etc/dnsmdcd.token").is_ok() {
            panic!("Could not create '/etc/dnsmdcd.token'!")
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
    let mut token_file = File::open("/etc/dnsmdcd.token").unwrap();
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

/**
 * Restarts the dnsmasq service and returns true in case of success,
 * false in case of failure. Can panic.
 * "Reloading" the service is not enough the parse /etc/custom.conf,
 * it has to be a restart.
 */
fn restart_dnsmasq() -> bool {
    let status = Command::new("systemctl")
        .arg("restart")
        .arg("dnsmasq.service")
        .status()
        .expect("Failed to spawn child process");
    
    match status.code() {
        Some(0) =>  true,
        _ => false
    }
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
        json.push_str("{\"name\":\"");
        json.push_str(&adr[0][..]);
        json.push_str("\",\"ip\":\"");
        json.push_str(&adr[1][..]);
        json.push_str("\"},");
    }

    json.pop();
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
            "{\"addresses\":[{\"name\":\"test1\",\"ip\":\"127.0.0.1\"},{\"name\":\"test2\",\"ip\":\"127.0.0.1\"}]}"
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
