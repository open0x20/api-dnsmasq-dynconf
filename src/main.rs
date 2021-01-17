use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::process::Command;
use std::fs::File;
use regex::Regex;

fn main() -> Result<(), i32> {
    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }

    Ok(())
}

/* The main procedure for every new request. Parses the request and acts
 * accordingly. Either lists all the contents of /etc/dnsmasq.d/custom.conf,
 * appends or deletes from it.
 */
fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let response_ok = "HTTP/1.1 200 OK\r\n\r\n";
    //let response_bad = "HTTP/1.1 400 Bad Request\r\n\r\n";
    let response_unauthorized = "HTTP/1.1 401 Unauthorized\r\n\r\n";
    //let response_internal_error = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
    // TODO error handling

    stream.read(&mut buffer).unwrap();
    let stream_contents = String::from_utf8_lossy(&buffer[..]);

    // Parse the request
    let query_params = parse_request(&stream_contents[..]).unwrap(); // todo

    // Check for authorization
    let mut token_file = File::open("/etc/dnsmasq-dynconf.token").unwrap();
    let mut token = String::new();
    let _ = token_file.read_to_string(&mut token);

    if token.trim() != query_params[3] {
        println!("{} != {}", token.as_str(), query_params[3]);
        let _ = stream.write((&response_unauthorized).as_bytes());
        return;
    }

    // Load all entries from /etc/dnsmasq.d/custom.conf
    let mut custom_file_read = File::open("/etc/dnsmasq.d/custom.conf").unwrap();
    let mut contents = String::new();
    let _ = custom_file_read.read_to_string(&mut contents);

    let entries_raw: Vec<&str> = contents.split_terminator("\n").collect();
    let mut entries: Vec<Vec<&str>> = Vec::new();

    for e in entries_raw {
        entries.push(parse_address_string(e));
    }

    // If it's a list simply return all entries
    if "list" == query_params[0] {
        let response_body: String = parse_address_vector_into_json_string(entries);
        let _ = stream.write((&response_ok).as_bytes());
        let _ = stream.write(response_body.as_bytes());

        return;
    }

    // Add an entry
    if "add" == query_params[0] {
        entries.push(vec![query_params[1], query_params[2]]);
        write_to_custom_file(parse_address_vector_into_address_string(entries));

        // Reload the dnsmasqd.service
        let _ = Command::new("/usr/bin/systemctl reload dnsmasqd.service").spawn();

        // Return a HTTP 200 OK response
        let _ = stream.write((&response_ok).as_bytes());

        return;
    }

    // Delete one or more existing entries
    if "delete" == query_params[0] {
        let mut indicies_to_delete: Vec<usize> = Vec::new();
        for i in 0..entries.len() {
            if (query_params[1] == entries[i][0]) && (query_params[2] == entries[i][1]) {
                indicies_to_delete.push(i);
            }
        }

        indicies_to_delete.reverse();
        for itd in indicies_to_delete {
            entries.remove(itd);
        }
        
        write_to_custom_file(parse_address_vector_into_address_string(entries));

        // Reload the dnsmasqd.service
        let _ = Command::new("/usr/bin/systemctl reload dnsmasqd.service").spawn();

        // Return a HTTP 200 OK response
        let _ = stream.write((&response_ok).as_bytes());

        return;
    }
}

/* Extracts the "method" and the required query parameters "name", "ip" and
 * "secret" as a vector of length 4. If a query parameter is missing an
 * appropriate http error code is returned.
 * The query parameters are allowed to be empty.
 *
 * The input string looks somewhat like this:
 * GET /list?name=test.myhost.de&ip=127.0.0.1&secret=ABCDEF HTTP/1.1
 * Host: localhost:11337
 * User-Agent: Mozilla/5.0 (X11; Fedora; Linux x86_64; rv:84.0) Gecko/20100101 Firefox/84.0
 * Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp
 * Accept-Language: en-US,en;q=0.5
 * Accept-Encoding: gzip, deflate
 * Connection: keep-alive
 * Upgrade-Insecure-Requests: 1
 * 
 * assert_eq!(parse_request(input), Ok(vec!["list", "test.myhost.de", "127.0.0.1", "ABCDEF"]));
 */
fn parse_request(request : &str) -> Result<Vec<&str>, i32> {
    // TODO static regex
    let method_regex: Regex = Regex::new(r"(^GET\s/list|^PUT\s/add|^POST\s/delete)").unwrap();
    let name_regex: Regex = Regex::new(r"name=[a-zA-Z0-9\.]*").unwrap();
    let ip_regex: Regex = Regex::new(r"ip=[0-9\.]*").unwrap();
    let secret_regex: Regex = Regex::new(r"secret=[a-zA-Z0-9]*").unwrap();

    let method_match = method_regex.find(request).ok_or(400)?;
    let name_match = name_regex.find(request).ok_or(400)?;
    let ip_match = ip_regex.find(request).ok_or(400)?;
    let secret_match = secret_regex.find(request).ok_or(400)?;

    let method = method_match.as_str().split_terminator("/").skip(1).next().unwrap_or("");
    let name = name_match.as_str().split_terminator("=").skip(1).next().unwrap_or("");
    let ip = ip_match.as_str().split_terminator("=").skip(1).next().unwrap_or("");
    let secret = secret_match.as_str().split_terminator("=").skip(1).next().unwrap_or("");

    println!("/{}?name={}&ip={}", method, name, ip);

    Ok(vec![method, name, ip, secret])
}

/* Extract the two values of a dnsmasq "address" entry string and return
 * them in a vector of length 2. It looks like this:
 *
 * assert_eq!(
 *   parse_address_string("address=/test.myhost.de/127.0.0.1"),
 *   vec!["test.myhost.de", "127.0.0.1"]
 * );
 */
fn parse_address_string(address: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = address.split_terminator("/").collect();
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
fn parse_address_vector_into_json_string(addresses: Vec<Vec<&str>>) -> String {
    let mut json: String = String::new();
    json.push_str("{\"addresses\":[");

    for adr in addresses {
        json.push_str("{\"address\":\"");
        json.push_str(adr[0]);
        json.push_str("\",\"ip\":\"");
        json.push_str(adr[1]);
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
fn parse_address_vector_into_address_string(addresses: Vec<Vec<&str>>) -> String {
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
    fn test_parse_request() {
        // Positive 

        // Check "list" endpoint
        let input = "GET /list?name=&ip=&secret=ABCDEF HTTP/1.1";
        assert_eq!(parse_request(input), Ok(vec!["list", "", "", "ABCDEF"]));

        // Check "add" endpoint
        let input = "PUT /add?name=test.myhost.de&ip=127.0.0.1&secret=ABCDEF HTTP/1.1";
        assert_eq!(parse_request(input), Ok(vec!["add", "test.myhost.de", "127.0.0.1", "ABCDEF"]));

        // Check "delete" endpoint
        let input = "POST /delete?name=test.myhost.de&ip=127.0.0.1&secret=ABCDEF HTTP/1.1";
        assert_eq!(parse_request(input), Ok(vec!["delete", "test.myhost.de", "127.0.0.1", "ABCDEF"]));

        // Additional query parameters should be ignored
        let input = "PUT /add?name=test.myhost.de&ip=127.0.0.1&secret=ABCDEF&test=true HTTP/1.1";
        assert_eq!(parse_request(input), Ok(vec!["add", "test.myhost.de", "127.0.0.1", "ABCDEF"]));

        // Missing query parameters should return Err(400)
        let input = "GET /list HTTP/1.1";
        assert_eq!(parse_request(input), Err(400));

        // Empty query parameters should return Err(400) for add and delete
        //let input = "PUT /add?name=&ip=&secret= HTTP/1.1";
        //assert_eq!(parse_request(input), Err(400));

        // Empty query parameters should return Err(400) for add and delete
        //let input = "POST /delete?name=&ip=&secret= HTTP/1.1";
        //assert_eq!(parse_request(input), Err(400));
    }

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