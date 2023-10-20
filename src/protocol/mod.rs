pub mod byte_packet_buffer;
pub mod dns_header;
pub mod dns_packet;
pub mod dns_question;
pub mod dns_record;
pub mod query_type;
pub mod result_code;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T> = anyhow::Result<T, Error>;
