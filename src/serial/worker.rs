use std::io::Read;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};

use crate::telemetry::packet::{self, Command, Telemetry};
use crate::telemetry::parser::StreamParser;

pub enum SerialCommand {
    Connect { port: String, baud: u32 },
    Disconnect,
    SendCommand(Command),
    SendCommandWithPayload(Command, Vec<u8>),
    SetPollInterval(u64),
    StartPolling,
    StopPolling,
}

pub enum SerialEvent {
    Connected(String),
    Disconnected,
    Error(String),
    Packet(Box<Telemetry>),
    PacketLost(u64),
    PortList(Vec<String>),
}

pub fn spawn(
    ctx: egui::Context,
) -> (Sender<SerialCommand>, Receiver<SerialEvent>) {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (evt_tx, evt_rx) = crossbeam_channel::unbounded();

    thread::spawn(move || {
        let mut port: Option<Box<dyn serialport::SerialPort>> = None;
        let mut parser = StreamParser::new();
        let mut read_buf = [0u8; 256];
        let mut last_port_scan = Instant::now();
        let port_scan_interval = Duration::from_secs(2);
        let mut poll_interval_ms: u64 = 1000;
        let mut polling_active = false;
        let mut last_poll = Instant::now();
        let mut next_telem_a = true;
        let mut seq_a: u8 = 0;
        let mut seq_b: u8 = 0;
        let mut last_recv_seq_a: Option<u8> = None;
        let mut last_recv_seq_b: Option<u8> = None;
        let mut packets_lost: u64 = 0;

        loop {
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    SerialCommand::Connect { port: name, baud } => {
                        match serialport::new(&name, baud)
                            .timeout(Duration::from_millis(100))
                            .open()
                        {
                            Ok(p) => {
                                port = Some(p);
                                parser = StreamParser::new();
                                last_poll = Instant::now();
                                seq_a = 0;
                                seq_b = 0;
                                last_recv_seq_a = None;
                                last_recv_seq_b = None;
                                packets_lost = 0;
                                let _ = evt_tx.send(SerialEvent::Connected(name));
                            }
                            Err(e) => {
                                let _ = evt_tx.send(SerialEvent::Error(e.to_string()));
                            }
                        }
                    }
                    SerialCommand::Disconnect => {
                        port = None;
                        let _ = evt_tx.send(SerialEvent::Disconnected);
                    }
                    SerialCommand::SendCommand(cmd) => {
                        if let Some(ref mut p) = port {
                            let frame = packet::build_command_frame(cmd);
                            if let Err(e) = std::io::Write::write_all(p.as_mut(), &frame) {
                                let _ = evt_tx.send(SerialEvent::Error(e.to_string()));
                            }
                        }
                    }
                    SerialCommand::SendCommandWithPayload(cmd, payload) => {
                        if let Some(ref mut p) = port {
                            let frame = packet::build_command_frame_with_payload(cmd, &payload);
                            if let Err(e) = std::io::Write::write_all(p.as_mut(), &frame) {
                                let _ = evt_tx.send(SerialEvent::Error(e.to_string()));
                            }
                        }
                    }
                    SerialCommand::SetPollInterval(ms) => {
                        poll_interval_ms = ms;
                    }
                    SerialCommand::StartPolling => {
                        polling_active = true;
                        last_poll = Instant::now();
                    }
                    SerialCommand::StopPolling => {
                        polling_active = false;
                    }
                }
            }

            if let Some(ref mut p) = port {
                if polling_active && last_poll.elapsed() >= Duration::from_millis(poll_interval_ms) {
                    last_poll = Instant::now();
                    let (cmd, seq) = if next_telem_a {
                        seq_a = seq_a.wrapping_add(1);
                        (Command::RequestTelemA, seq_a)
                    } else {
                        seq_b = seq_b.wrapping_add(1);
                        (Command::RequestTelemB, seq_b)
                    };
                    next_telem_a = !next_telem_a;
                    let frame = packet::build_command_frame_with_payload(cmd, &[seq]);
                    if let Err(e) = std::io::Write::write_all(p.as_mut(), &frame) {
                        let _ = evt_tx.send(SerialEvent::Error(e.to_string()));
                    }
                }

                let available = p.bytes_to_read().unwrap_or(0);
                if available > 0 {
                    match p.read(&mut read_buf) {
                        Ok(n) if n > 0 => {
                            let packets = parser.feed(&read_buf[..n]);
                            for pkt in packets {
                                let (last_seq, _sent_seq) = match pkt.source {
                                    packet::TelemetrySource::A => (&mut last_recv_seq_a, seq_a),
                                    packet::TelemetrySource::B => (&mut last_recv_seq_b, seq_b),
                                    _ => (&mut last_recv_seq_a, seq_a),
                                };
                                if let Some(prev) = *last_seq {
                                    let gap = pkt.seq.wrapping_sub(prev).wrapping_sub(1) as u64;
                                    if gap > 0 && gap < 128 {
                                        packets_lost += gap;
                                        let _ = evt_tx.send(SerialEvent::PacketLost(packets_lost));
                                    }
                                }
                                *last_seq = Some(pkt.seq);
                                let _ = evt_tx.send(SerialEvent::Packet(Box::new(pkt)));
                            }
                            ctx.request_repaint();
                        }
                        Ok(_) => {}
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                        Err(e) => {
                            let _ = evt_tx.send(SerialEvent::Error(e.to_string()));
                            port = None;
                            let _ = evt_tx.send(SerialEvent::Disconnected);
                        }
                    }
                } else {
                    thread::sleep(Duration::from_millis(1));
                }
            } else {
                thread::sleep(Duration::from_millis(50));
            }

            if last_port_scan.elapsed() >= port_scan_interval {
                last_port_scan = Instant::now();
                if let Ok(ports) = serialport::available_ports() {
                    let names: Vec<String> = ports.into_iter().map(|p| p.port_name).collect();
                    let _ = evt_tx.send(SerialEvent::PortList(names));
                }
            }
        }
    });

    (cmd_tx, evt_rx)
}
