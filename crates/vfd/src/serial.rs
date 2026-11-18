//! The real serial ports (the `serial` feature): a USB-RS485 adapter shows up as a COM port (an FTDI
//! FT232RL on the Waveshare adapter: USB id 0403:6001).

use crate::link::{Bus, Line, Parity, Port};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

pub struct SerialBus;

/// FTDI's USB vendor id (the Waveshare adapter's FT232RL).
const FTDI: u16 = 0x0403;

impl Bus for SerialBus {
    /// Only USB serial ports are tried on their own (never Bluetooth or a built-in port, where a stray Modbus
    /// request could reach something else); if any is an FTDI one, only those.
    fn ports(&self) -> Vec<(String, String)> {
        let all = serialport::available_ports().unwrap_or_default();
        let usb: Vec<(String, String, bool)> = all
            .into_iter()
            .filter_map(|p| match p.port_type {
                serialport::SerialPortType::UsbPort(u) => {
                    let desc = [u.manufacturer.clone(), u.product.clone()].into_iter().flatten().collect::<Vec<_>>().join(" ");
                    Some((p.port_name, format!("{desc} ({:04x}:{:04x})", u.vid, u.pid), u.vid == FTDI))
                }
                _ => None,
            })
            .collect();
        let ftdi = usb.iter().any(|p| p.2);
        usb.into_iter().filter(|p| p.2 || !ftdi).map(|p| (p.0, p.1)).collect()
    }

    fn open(&self, name: &str, line: Line) -> Result<Box<dyn Port>, String> {
        let port = serialport::new(name, line.baud)
            .data_bits(serialport::DataBits::Eight)
            .parity(match line.parity {
                Parity::None => serialport::Parity::None,
                Parity::Even => serialport::Parity::Even,
                Parity::Odd => serialport::Parity::Odd,
            })
            .stop_bits(if line.stop_bits == 2 { serialport::StopBits::Two } else { serialport::StopBits::One })
            .flow_control(serialport::FlowControl::None)
            .timeout(Duration::from_millis(10))
            .open()
            .map_err(|e| e.to_string())?;
        Ok(Box::new(SerialPort { port, baud: line.baud }))
    }
}

struct SerialPort {
    port: Box<dyn serialport::SerialPort>,
    baud: u32,
}

impl Port for SerialPort {
    fn exchange(&mut self, request: &[u8], expect: usize) -> Vec<u8> {
        let _ = self.port.clear(serialport::ClearBuffer::All);
        if self.port.write_all(request).and_then(|_| self.port.flush()).is_err() {
            return vec![];
        }
        // the request and the answer on the wire (11 bits a byte), plus the drive's own turnaround
        let wire = (request.len() + expect) as f64 * 11.0 / self.baud as f64;
        let end = Instant::now() + Duration::from_secs_f64(wire) + Duration::from_millis(120);
        let mut got = Vec::with_capacity(expect);
        let mut buf = [0u8; 64];
        while got.len() < expect && Instant::now() < end {
            match self.port.read(&mut buf) {
                Ok(n) => got.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(_) => break,
            }
            // an exception answer is complete at 5 bytes
            if got.len() >= crate::modbus::EXCEPTION_LEN && got[1] & 0x80 != 0 {
                break;
            }
        }
        got
    }
}
