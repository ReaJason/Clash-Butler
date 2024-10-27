use base64::prelude::BASE64_STANDARD;
use base64::Engine;

pub fn base64decode(content: &str) -> String {
    let mut padded_content = content.to_string();
    while padded_content.len() % 4 != 0 {
        padded_content.push('=');
    }
    match BASE64_STANDARD.decode(padded_content.as_bytes()) {
        Ok(data) => {
            String::from_utf8(data).unwrap()
        }
        Err(_) => {
            content.to_string()
        }
    }
}

pub fn base64encode(content: String) -> String {
    let b: &[u8] = content.as_bytes();
    BASE64_STANDARD.encode(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64() {
        let str = "aes-256-gcm:Q1GUZ7VDPZOASC9H";
        println!("{}", base64decode(str));
    }

    #[test]
    fn test_base64decode() {
        let str = base64decode(String::from("aGVsbG8=").as_str());
        assert_eq!(str, String::from("hello"))
    }

    #[test]
    fn test_base64decode_error() {
        let str = base64decode(String::from("aGVsbG8").as_str());
        assert_eq!(str, String::from("hello"))
    }
}