use std::{net::Ipv4Addr, sync::Arc};

use crate::{
    config::Config,
    dns::recursive_lookup,
    protocol::{
        byte_packet_buffer::BytePacketBuffer, dns_packet::DnsPacket, dns_question::DnsQuestion,
        dns_record::DnsRecord, result_code::ResultCode, Result,
    },
    rules::{match_rule, Rule, A_APPEND, A_DENY},
};

use super::peer::UdpPeer;

pub async fn handle_query(
    config: &Config,
    rules: &Vec<Rule>,
    question: &DnsQuestion,
    out: &mut DnsPacket,
) {
    // Try match rules.
    let rule_matched = match_rule(rules, &question.name);

    if rule_matched.is_some() {
        let rule_matched = rule_matched.unwrap();

        match rule_matched.action {
            A_DENY => {
                out.header.rescode = ResultCode::NXDOMAIN;
                return;
            }
            A_APPEND => {
                out.header.rescode = ResultCode::NOERROR;
                out.header.recursion_desired = false;
                out.header.recursion_available = false;

                let raw_addr = rule_matched.value.unwrap_or("127.0.0.1".to_string());

                let domain = question.name.clone();
                let addr = raw_addr.parse::<Ipv4Addr>().unwrap();
                let ttl = 53000;

                let record = DnsRecord::A { domain, addr, ttl };
                out.answers.push(record);
                return;
            }
            _ => {}
        }
    }

    // Try mirror.
    let mirror_enabled = config.mirror.enabled;
    let mirror_ns = config.mirror.server.as_str();

    if mirror_enabled {
        let result = recursive_lookup(mirror_ns, &question.name, question.qtype);

        if let Ok(result) = result {
            out.header.rescode = result.header.rescode;

            if result.header.rescode == ResultCode::NOERROR {
                for rec in result.answers {
                    out.answers.push(rec);
                }

                for rec in result.authorities {
                    out.authorities.push(rec);
                }

                for rec in result.resources {
                    out.resources.push(rec);
                }
            }
        } else {
            out.header.rescode = ResultCode::SERVFAIL;
        }
    }
}

pub async fn handle_request(
    config: &Config,
    rules: &Vec<Rule>,
    peer: &Arc<UdpPeer>,
    buffer: &mut BytePacketBuffer,
) -> Result<()> {
    let mut request = DnsPacket::from_buffer(buffer)?;

    let mut packet = DnsPacket::new();
    packet.header.id = request.header.id;
    packet.header.recursion_desired = true;
    packet.header.recursion_available = true;
    packet.header.response = true;

    if let Some(question) = request.questions.pop() {
        log::info!(
            "Client {} requested {:?} {}",
            peer.addr,
            question.qtype,
            question.name,
        );

        packet.questions.push(question.clone());
        handle_query(config, rules, &question, &mut packet).await;
    } else {
        packet.header.rescode = ResultCode::FORMERR;
    }

    let mut res_buffer = BytePacketBuffer::new();
    packet.write(&mut res_buffer)?;

    let len = res_buffer.pos();
    let data = res_buffer.get_range(0, len)?;

    peer.send(data).await?;
    Ok(())
}
