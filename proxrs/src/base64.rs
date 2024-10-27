use base64::prelude::BASE64_STANDARD;
use base64::Engine;

pub fn base64decode(content: &str) -> String {
    let cleaned = content.replace("_", "/");
    let padded = match cleaned.len() % 4 {
        0 => cleaned,
        n => cleaned + &"=".repeat(4 - n),
    };
    match BASE64_STANDARD.decode(padded.as_bytes()) {
        Ok(data) => String::from_utf8(data).unwrap(),
        Err(_) => content.to_string(),
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
    fn test_base64_invalid() {
        let str = "anAtYW00OC02LmVxbm9kZS5uZXQ6ODA4MTpvcmlnaW46YWVzLTI1Ni1jZmI6dGxzMS4yX3RpY2tldF9hdXRoOlpVRnZhMkpoUkU0Mi8_Z3JvdXA9Y0hKdmVIbHdiMjlzYzNNdWFHVnliMnQxWVhCd0xtTnZiUSUzRCUzRCZvYmZzcGFyYW09JnByb3RvcGFyYW09";
        assert_eq!("jp-am48-6.eqnode.net:8081:origin:aes-256-cfb:tls1.2_ticket_auth:ZUFva2JhRE42/?group=cHJveHlwb29sc3MuaGVyb2t1YXBwLmNvbQ%3D%3D&obfsparam=&protoparam=", base64decode(str));
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
