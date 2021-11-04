use std::collections::HashMap;
use std::num::ParseIntError;

pub enum HttpVerb {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
    PATCH,
}


pub struct HttpRequest {
    pub header: HttpRequestHeader,
    body: Option<Vec<u8>>,
}

pub struct HttpRequestHeader {
    pub route: String,
    pub verb: HttpVerb,
    pub content_length: i32,
    pub headers: HashMap<String, String>,
    pub http_version: String,

}

pub struct HttpResponse {
    pub http_version: String,
    pub code: i16,
    pub content_type: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}


impl HttpVerb {
    pub fn from_str(data: &str) -> Result<HttpVerb, &'static str> {
        match data.to_uppercase().as_str() {
            "GET" => Ok(HttpVerb::GET),
            "HEAD" => Ok(HttpVerb::HEAD),
            "POST" => Ok(HttpVerb::POST),
            "PUT" => Ok(HttpVerb::PUT),
            "DELETE" => Ok(HttpVerb::DELETE),
            "CONNECT" => Ok(HttpVerb::CONNECT),
            "PATCH" => Ok(HttpVerb::PATCH),
            "OPTIONS" => Ok(HttpVerb::OPTIONS),
            "TRACE" => Ok(HttpVerb::TRACE),
            _ => Err("Unknown http verb")
        }
    }
}

impl HttpRequest {

    pub fn create(header: HttpRequestHeader, body: Option<Vec<u8>>) -> Result<HttpRequest, &'static str> {
        Ok(HttpRequest {
            header,
            body
        })
    }
}

impl HttpRequestHeader {
    pub fn create_from_buffer(buffer: [u8; 4096]) -> Result<(HttpRequestHeader, usize), &'static str> {
        for i in 0..buffer.len() {
            if i > 4 &&
                buffer[i] == 10 &&
                buffer[i - 1] == 13 &&
                buffer[i - 2] == 10 &&
                buffer[i - 3] == 13 {
                // \r\n\r\n found, after this its the body.
                let header = String::from_utf8_lossy(&buffer[0..i]).into_owned();

                //println!("{}", header);

                let request = HttpRequestHeader::parse_from_string(header)?;

                return Ok((request, i));
            }
        }

        Err("Request header larger than buffer")
    }

    pub fn parse_from_string(data: String) -> Result<HttpRequestHeader, &'static str> {
        let split_header: Vec<&str> = data.split("\r\n").collect();

        let mut headers = HashMap::new();

        let mut content_length: i32 = 0;

        let split_status_line: Vec<&str> = split_header[0].split(" ").collect();

        let verb = HttpVerb::from_str(split_status_line[0])?;
        let route = String::from(split_status_line[1]);
        let http_version = String::from(split_status_line[2]);

        for i in 1..split_header.len() {
            //println!("Head: {}", split_header[i]);

            let split_item: Vec<&str> = split_header[i].split(": ").collect();

            // If the split item has more than 1 item, add a header.
            if split_item.len() > 1 {

                let k = String::from(split_item[0]).to_uppercase();
                let v = String::from(split_item[1]);

                // If the header item is `Content-Length` set it as such.
                if k == "CONTENT-LENGTH" {
                    match v.parse::<i32>() {
                        Ok(i) => content_length = i,
                        Err(_) => {}
                    }
                }

                headers.insert(k, v);
            }
        }

        Ok(HttpRequestHeader {
            route,
            verb,
            content_length,
            headers,
            http_version
        })
    }
}

impl HttpResponse {

    pub fn create(code: i16, content_type: String, body: Option<Vec<u8>>) -> HttpResponse {

        let http_version = String::from("HTTP/1.1");

        // Map the headers.
        let mut mapped_headers: HashMap<String, String> = HashMap::new();

        let len = match &body {
            None => 0,
            Some(b) => b.len()
        };

        // Add any standardized headers.
        mapped_headers.insert("Server".to_string(), "Psionic 0.0.1".to_string());
        mapped_headers.insert("Content-Length".to_string(), format!("{}", len));
        mapped_headers.insert("Connection".to_string(), "Closed".to_string());
        mapped_headers.insert("Content-Type".to_string(), content_type.clone());

        // Add any other headers.
        //for header in headers {
        // TODO check for existing header key, then handle overriding (or not) the value.
        //    mapped_headers.insert(header.key, header.value);
        //}

        HttpResponse {
            http_version,
            code,
            content_type,
            headers: mapped_headers,
            body,
        }
    }

    pub fn to_bytes(&mut self) -> Vec<u8> {
        let response_type = get_response_type_str(self.code);

        // Create the header.
        let mut header_string = String::new();

        header_string.push_str(&self.http_version);
        header_string.push(' ');
        header_string.push_str(&self.code.to_string());
        header_string.push(' ');
        header_string.push_str(response_type);

        header_string.push_str("\r\n");

        for header in &self.headers {
            header_string.push_str(&header.0);
            header_string.push_str(": ");
            header_string.push_str(&header.1);
            header_string.push_str("\r\n");
        }

        header_string.push_str("\r\n");

        // Get the bytes for the header and append the response body.
        let mut bytes = Vec::from(header_string.as_bytes());


        if let Some(b) = &self.body {

            let mut body = b.clone();

            bytes.append(&mut body);
        }

        bytes
    }
}

fn get_response_type_str(code: i16) -> &'static str {
    match code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Error",
        _ => "Unknown",
    }
}