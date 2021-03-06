extern crate sha1;
use self::sha1::Sha1;

use dns_coding::{DecPacket, Decoder, EncPacket, Encoder, dns_encode};
use dns_proto::{Domain, Message, Record, RecordHeader};

use super::record_code::{get_record_code};
use super::util::{is_api_query, domain_ends_with, domain_part_lowercase};

/// Check if a DNS message is an establishment API call.
pub fn is_establish_query(query: &Message) -> bool {
    is_api_query(query, 'e')
}

/// Produce a response message for an establishment request.
///
/// # Arguments
///
/// * `query` - The query to respond to.
/// * `host` - The root domain name of the server.
/// * `resp` - The response to encode.
pub fn establish_response(
    query: &Message,
    host: &Domain,
    resp: EstablishResponse
) -> Result<Message, String> {
    let equery = EstablishQuery::from_query(query, host)?;
    let question = &query.questions[0];
    let code = get_record_code(question.record_type, &equery.response_encoding)
        .ok_or("no response encoding".to_owned())?;
    let body = code.encode_body(&dns_encode(&resp)?)?;
    let mut result = query.clone();
    result.answers.push(Record{
        header: RecordHeader{
            domain: question.domain.clone(),
            record_type: question.record_type,
            record_class: question.record_class,
            ttl: 0
        },
        body: body
    });
    result.header.answer_count = 1;
    result.header.is_response = true;
    Ok(result)
}

/// Produce proof that we know the given password, using the current epoch
/// time in seconds.
pub fn password_proof(password: &str, cur_time: u64) -> u64 {
    let mut sh = Sha1::new();
    sh.update(format!("{}{}{}", password, cur_time, password).as_bytes());
    let hash = sh.digest().bytes();
    ((hash[0] as u64) << 56) | ((hash[1] as u64) << 48) | ((hash[2] as u64) << 40) |
        ((hash[3] as u64) << 32) | ((hash[4] as u64) << 24) | ((hash[5] as u64) << 16) |
        ((hash[6] as u64) << 8) | (hash[7] as u64)
}

/// The contents of an establishment query.
#[derive(Debug, PartialEq)]
pub struct EstablishQuery {
    pub response_encoding: String,
    pub mtu: u16,
    pub name_encoding: String,
    pub query_window: u16,
    pub response_window: u16,
    pub proof: u64,
    pub port: u16,
    pub host: Domain
}

impl EstablishQuery {
    /// Decode an establishment query.
    ///
    /// # Arguments
    ///
    /// * `query` - The query to decode.
    /// * `host` - The root domain name of the server.
    pub fn from_query(query: &Message, host: &Domain) -> Result<EstablishQuery, String> {
        if !is_establish_query(query) {
            return Err("not an establish query".to_owned());
        }
        EstablishQuery::from_domain(&query.questions[0].domain, host)
    }

    fn from_domain(domain: &Domain, host: &Domain) -> Result<EstablishQuery, String> {
        if !domain_ends_with(domain, host) {
            return Err("incorrect host domain".to_owned());
        }
        if domain.parts().len() - host.parts().len() < 8 {
            return Err("not enough labels".to_owned());
        }
        let response_encoding = domain_part_lowercase(&domain.parts()[0])
            .chars().skip(1).collect();
        let mtu = domain.parts()[1].parse();
        let name_encoding = domain_part_lowercase(&domain.parts()[2]);
        let query_window = domain.parts()[3].parse();
        let response_window = domain.parts()[4].parse();
        let proof = u64::from_str_radix(&domain.parts()[5], 16);
        let port = domain.parts()[6].parse();
        let host = &domain.parts()[7..(domain.parts().len() - host.parts().len())];
        if mtu.is_err() || query_window.is_err() || response_window.is_err() || proof.is_err() ||
            port.is_err() {
            Err("invalid number in domain".to_owned())
        } else {
            Ok(EstablishQuery{
                response_encoding: response_encoding,
                mtu: mtu.unwrap(),
                name_encoding: name_encoding,
                query_window: query_window.unwrap(),
                response_window: response_window.unwrap(),
                proof: proof.unwrap(),
                port: port.unwrap(),
                host: Domain::from_parts(host.to_vec())?
            })
        }
    }

    /// Encode the request into a domain name, given the root domain name of the
    /// server, `host`.
    pub fn to_domain(&self, host: &Domain) -> Result<Domain, String> {
        let mut parts = Vec::new();
        parts.push(format!("e{}", self.response_encoding));
        macro_rules! push_fmt {
            ( $($x:expr),* ) => { { $(parts.push(format!("{}", $x));)* } }
        }
        push_fmt!(self.mtu, self.name_encoding, self.query_window, self.response_window);
        parts.push(format!("{:x}", self.proof));
        push_fmt!(self.port);
        parts.extend(self.host.parts().to_vec());
        parts.extend(host.parts().to_vec());
        Domain::from_parts(parts)
    }

    /// Check the password proof in the query.
    ///
    /// # Arguments
    ///
    /// * `password` - The correct password.
    /// * `cur_time` - The current epoch time, in seconds.
    /// * `window` - The number of seconds by which the client's clock is allowed
    ///   to be off from `cur_time`. The higher this value, the slower the check.
    pub fn check_proof(&self, password: &str, cur_time: u64, window: u64) -> bool {
        for i in (cur_time - window)..(cur_time + window) {
            if self.proof == password_proof(password, i) {
                return true;
            }
        }
        false
    }
}

/// A response to an establishment query.
#[derive(Debug, PartialEq)]
pub enum EstablishResponse {
    Success{id: u16, seq: u32},
    Failure(String),
    Unknown(u8)
}

impl Decoder for EstablishResponse {
    fn dns_decode(packet: &mut DecPacket) -> Result<EstablishResponse, String> {
        Ok(match u8::dns_decode(packet)? {
            0 => {
                let session_id = Decoder::dns_decode(packet)?;
                let seq_num = Decoder::dns_decode(packet)?;
                EstablishResponse::Success{id: session_id, seq: seq_num}
            },
            1 => {
                let size = packet.remaining();
                let raw = packet.read_bytes(size)?;
                EstablishResponse::Failure(String::from(String::from_utf8_lossy(&raw)))
            },
            x => {
                let size = packet.remaining();
                packet.read_bytes(size)?;
                EstablishResponse::Unknown(x)
            }
        })
    }
}

impl Encoder for EstablishResponse {
    fn dns_encode(&self, packet: &mut EncPacket) -> Result<(), String> {
        match self {
            &EstablishResponse::Success{id: ref session_id, seq: ref seq_num} => {
                0u8.dns_encode(packet)?;
                session_id.dns_encode(packet)?;
                seq_num.dns_encode(packet)
            },
            &EstablishResponse::Failure(ref message) => {
                1u8.dns_encode(packet)?;
                message.as_bytes().to_vec().dns_encode(packet)
            },
            &EstablishResponse::Unknown(_) => {
                Err("cannot encode unknown establish response".to_owned())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_encoding() {
        let query = EstablishQuery{
            response_encoding: "raw".to_owned(),
            mtu: 123,
            name_encoding: "b64".to_owned(),
            query_window: 64,
            response_window: 32,
            proof: 0x913379,
            port: 1337,
            host: "foo.bob.com".parse().unwrap()
        };
        let encoded = query.to_domain(&"baz.proxy.com".parse().unwrap()).unwrap();
        let expected = "eraw.123.b64.64.32.913379.1337.foo.bob.com.baz.proxy.com";
        assert_eq!(expected.parse::<Domain>().unwrap(), encoded);
    }

    #[test]
    fn query_decoding() {
        let query = EstablishQuery::from_domain(
            &"eraw.123.b64.64.32.913379.1337.foo.bob.com.baz.proxy.com".parse().unwrap(),
            &"baz.proxy.com".parse().unwrap()
        ).unwrap();
        assert_eq!(query, EstablishQuery{
            response_encoding: "raw".to_owned(),
            mtu: 123,
            name_encoding: "b64".to_owned(),
            query_window: 64,
            response_window: 32,
            proof: 0x913379,
            port: 1337,
            host: "foo.bob.com".parse().unwrap()
        });
    }
}
