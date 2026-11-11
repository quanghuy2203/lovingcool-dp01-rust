//! Protocol for verified Hongtai CDC-ACM variants only; DP-01 USB ID must be checked.
use std::{io::{Read, Write}, time::Duration};
use serialport::SerialPort;

pub struct Panel {
    serial: Box<dyn SerialPort>,
    pub width: u32,
    pub height: u32,
    pub model: String,
    pub firmware: String,
    wrapped: bool,
    spi: bool,
    max_kb: usize,
}

pub fn candidates() -> Vec<String> {
    serialport::available_ports().unwrap_or_default().into_iter().filter_map(|p| {
        match p.port_type {
            serialport::SerialPortType::UsbPort(info) if info.vid == 0x33c3 && [0x7791, 0x7788].contains(&info.pid) => Some(p.port_name),
            _ => None,
        }
    }).collect()
}
fn checksum(bytes: &[u8]) -> [u8; 2] {
    (bytes.iter().fold(0u32, |a,b| a + *b as u32) as u16).to_le_bytes()
}
fn packet(cmd: u8, payload: &[u8]) -> Vec<u8> {
    let mut out=vec![0x55,0xaa];
    out.extend_from_slice(&((payload.len()+7) as u16).to_le_bytes());
    out.push(cmd); out.extend_from_slice(payload);
    out.extend_from_slice(&checksum(&out)); out
}
impl Panel {
    pub fn connect(port: &str) -> Result<Self, String> {
        // Safety: never guess protocols for arbitrary COM devices.
        if !candidates().iter().any(|p| p==port) { return Err("COM không có VID/PID Hongtai đã biết (33C3:7791/7788).".into()); }
        let serial=serialport::new(port, 2_000_000).timeout(Duration::from_millis(450)).open().map_err(|e|e.to_string())?;
        let mut p=Self {serial,width:480,height:480,model:String::new(),firmware:String::new(),wrapped:false,spi:false,max_kb:50};
        p.serial.write_all(&[0xff,0xd9,0xff,0xd9,0,0,0,0]).map_err(|e|e.to_string())?;
        std::thread::sleep(Duration::from_millis(300));
        p.serial.clear(serialport::ClearBuffer::Input).ok();
        p.serial.write_all(&packet(0x06,&[])).map_err(|e|e.to_string())?;
        let mut response=Vec::new(); let mut temp=[0u8;4096];
        for _ in 0..7 {
            match p.serial.read(&mut temp) {
                Ok(n) if n>0 => { response.extend_from_slice(&temp[..n]);
                    if response.len()>=7 && response.starts_with(&[0x55,0xaa]) {
                        let len=u16::from_le_bytes([response[2],response[3]]) as usize;
                        if response.len()>=len { break; }
                    }
                }, _=>(),
            }
        }
        if !response.starts_with(&[0x55,0xaa]) || response.len()<8 {return Err("Không đọc được thông tin LCD; chưa xác minh giao thức.".into());}
        let len=u16::from_le_bytes([response[2],response[3]]) as usize;
        if len > response.len() || len < 7 || checksum(&response[..len-2]) != [response[len-2],response[len-1]] {
            return Err("Phản hồi LCD sai độ dài/checksum.".into());
        }
        let json:serde_json::Value=serde_json::from_slice(&response[5..len-2]).map_err(|e|e.to_string())?;
        let info=json.get("data").unwrap_or(&json);
        p.width=info["width"].as_u64().unwrap_or(0) as u32;
        p.height=info["height"].as_u64().unwrap_or(0) as u32;
        if !(64..=2048).contains(&p.width) || !(64..=2048).contains(&p.height) {return Err("Kích thước panel không hợp lệ.".into());}
        p.model=info["model"].as_str().unwrap_or("unknown").to_string();
        p.firmware=info["version"].as_str().unwrap_or("unknown").to_string();
        let ver=p.firmware.trim_start_matches("Ver").parse::<f32>().map_err(|_|"Không xác định phiên bản firmware; từ chối gửi frame để tránh lỗi màn hình.".to_string())?;
        p.wrapped=ver>2.8;
        let m=p.model.to_lowercase();
        p.spi=m.contains("2.99")||m.contains("txw813-st7789-2.8inch")||m.contains("qspi");
        if p.spi {return Err("Panel SPI/RGB565 chưa được hỗ trợ an toàn trong bản này.".into());}
        p.max_kb=if p.wrapped {80}else{50};
        Ok(p)
    }
    pub fn keepalive(&mut self) -> Result<(),String> { self.serial.write_all(&packet(0x11,&[])).map_err(|e|e.to_string()) }
    pub fn brightness(&mut self, b:u8) -> Result<(),String> { self.serial.write_all(&packet(0x03,&[b.min(100)])).map_err(|e|e.to_string()) }
    pub fn send(&mut self, rgb:&image::RgbImage) -> Result<(),String> {
        let resized=image::imageops::resize(rgb,self.width,self.height,image::imageops::FilterType::Triangle);
        let mut encoded=Vec::new();
        for quality in [85u8,70,55,40,25,12] {
            encoded.clear();
            let mut enc=image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded,quality);
            enc.encode_image(&resized).map_err(|e|e.to_string())?;
            if encoded.len() <= self.max_kb*1024 {break;}
        }
        if encoded.len()>self.max_kb*1024 { return Err("JPEG vượt giới hạn firmware; giảm độ phân giải/độ phức tạp wallpaper.".into()); }
        if self.wrapped {
            let mut frame=(encoded.len() as u32).to_le_bytes().to_vec();
            frame.extend_from_slice(&encoded);
            frame.extend_from_slice(&checksum(&frame));
            for chunk in frame.chunks(20*1024) {self.serial.write_all(chunk).map_err(|e|e.to_string())?;}
        } else {
            for chunk in encoded.chunks(20*1024) {self.serial.write_all(chunk).map_err(|e|e.to_string())?;}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn command_checksum(){ assert_eq!(packet(0x11,&[]), vec![0x55,0xaa,0x07,0x00,0x11,0x17,0x01]); }
}
