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
}

pub enum SerialEvent {
    Connected(String),
    Disconnected,
    Error(String),
    Packet(Box<Telemetry>),
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
                }
            }

            if let Some(ref mut p) = port {
                let available = p.bytes_to_read().unwrap_or(0);
                if available > 0 {
                    match p.read(&mut read_buf) {
                        Ok(n) if n > 0 => {
                            let packets = parser.feed(&read_buf[..n]);
                            for pkt in packets {
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
                    thread::sleep(Duration::from_millis(10));
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
