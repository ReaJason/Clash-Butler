use base64::prelude::BASE64_STANDARD;
use base64::{DecodeError, Engine};

pub fn base64decode(content: &str) -> Result<String, DecodeError> {
    let mut padded_content = content.to_string();
    while padded_content.len() % 4 != 0 {
        padded_content.push('=');
    }
    match BASE64_STANDARD.decode(padded_content.as_bytes()) {
        Ok(data) => {
            Ok(String::from_utf8(data).unwrap())
        }
        Err(e) => {
            Err(e)
        }
    }
}

pub fn base64encode(content: String) -> String {
    let b: &[u8] = content.as_bytes();
    BASE64_STANDARD.encode(b)
}

#[cfg(test)]
mod tests {
    use wasm_web_reajason::hello;
    use super::*;

    #[test]
    fn test_base64() {
        // println!("{}", base64decode(String::from("c3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDA2NzYjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAxMjAuMjMyLjczLjY4OjQ3MDM0IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMTIwLjIzMi43My42ODo0MzI5NiMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDQ0MjEjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAxMjAuMjMyLjczLjY4OjQzMDg4IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMTIwLjIzMi43My42ODo0NzE1MCMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDM2NjQjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAxMjAuMjMyLjczLjY4OjQwMTA3IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMTIwLjIzMi43My42ODo0MTgzMiMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMzIuNzMuNjg6NDM2NDEjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAyMTEuOTkuMTAyLjIyNDo0MDY3NiMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDIxMS45OS4xMDIuMjI0OjQ3MDM0IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMjExLjk5LjEwMi4yMjQ6NDMyOTYjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAyMTEuOTkuMTAyLjIyNDo0NDQyMSMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDIxMS45OS4xMDIuMjI0OjQzMDg4IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMjExLjk5LjEwMi4yMjQ6NDcxNTAjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAyMTEuOTkuMTAyLjIyNDo0MzY2NCMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDIxMS45OS4xMDIuMjI0OjQwMTA3IyVGMCU5RiU4NyVBRCVGMCU5RiU4NyVCMEhLDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMjExLjk5LjEwMi4yMjQ6NDE4MzIjJUYwJTlGJTg3JUFEJUYwJTlGJTg3JUIwSEsNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAyMTEuOTkuMTAyLjIyNDo0MzY0MSMlRjAlOUYlODclQUQlRjAlOUYlODclQjBISw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMDQuOTQuMzA6NDYxMDEjJUYwJTlGJTg3JUJBJUYwJTlGJTg3JUI4VVMNCnNzOi8vWVdWekxURXlPQzFuWTIwNlpEbGpOVGMzTXpJNFptSXpORGxtWlE9PUAxMjAuMjA0Ljk0LjMwOjQzNzkxIyVGMCU5RiU4NyVCQSVGMCU5RiU4NyVCOFVTDQpzczovL1lXVnpMVEV5T0MxblkyMDZaRGxqTlRjM016STRabUl6TkRsbVpRPT1AMTIwLjIwNC45NC4zMDo0MjI4NSMlRjAlOUYlODclQkElRjAlOUYlODclQjhVUw0Kc3M6Ly9ZV1Z6TFRFeU9DMW5ZMjA2WkRsak5UYzNNekk0Wm1Jek5EbG1aUT09QDEyMC4yMDQuOTQuMzA6NDY4MjQjJUYwJTlGJTg3JUJBJUYwJTlGJTg3JUI4VVMNCg").as_str()).unwrap());
        println!("{}", base64decode("eyJ2IjoiMiIsInBzIjoiXHU1MjY5XHU0ZjU5XHU2ZDQxXHU5MWNmXHVmZjFhNzkuNTEgR0IiLCJhZGQiOiJjZG5jZG5jZG5jZG4uNzg0NjU0Lnh5eiIsInBvcnQiOiIyMDUyIiwiaWQiOiIzZWE1NzhjNi0xZWFhLTRlMTUtYmZlMS05Zjc1N2I1OGU4ZjIiLCJhaWQiOiIwIiwibmV0Ijoid3MiLCJ0eXBlIjoibm9uZSIsImhvc3QiOiJjYS1jZmNkbi5haWt1bmFwcC5jb20iLCJwYXRoIjoiXC9pbmRleD9lZD0yMDQ4IiwidGxzIjoiIn0=").unwrap());
    }

    #[test]
    fn test_base64decode() {
        match base64decode(String::from("aGVsbG8=").as_str()) {
            Ok(str) => {
                assert_eq!(str, String::from("hello"))
            }
            Err(_) => {
                assert!(false)
            }
        }
    }

    #[test]
    fn test_base64encode() {
        hello("123")
    }

    #[test]
    fn test_base64decode_error() {
        let str = base64decode(String::from("aGVsbG8").as_str()).unwrap();
        assert_eq!(str, String::from("hello"))
    }
}