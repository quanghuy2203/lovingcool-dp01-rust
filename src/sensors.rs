use std::{io::{Read,Write},net::TcpStream,time::Duration};
use sysinfo::{System, Disks, Networks};
#[derive(Clone,Default)]
pub struct Stats {
 pub cpu:f32, pub ram:f32, pub used_ram:f32, pub total_ram:f32,
 pub disk:f32, pub net_down:u64, pub net_up:u64,
 pub cpu_temp:Option<f32>, pub gpu_temp:Option<f32>, pub gpu_load:Option<f32>,
 pub cpu_name:String, pub os:String, pub uptime:u64,
}
pub struct Monitor { sys:System, disks:Disks, nets:Networks }
impl Monitor {
 pub fn new()->Self {let mut sys=System::new_all();sys.refresh_cpu_usage(); Self{sys,disks:Disks::new_with_refreshed_list(),nets:Networks::new_with_refreshed_list()} }
 pub fn sample(&mut self, sensors_http:bool)->Stats {
   self.sys.refresh_cpu_usage(); self.sys.refresh_memory();
   self.disks.refresh(true);self.nets.refresh(true);
   let total=self.sys.total_memory() as f32; let used=self.sys.used_memory() as f32;
   let dt:u64=self.disks.list().iter().map(|d|d.total_space()).sum();
   let du:u64=self.disks.list().iter().map(|d|d.total_space().saturating_sub(d.available_space())).sum();
   let mut s=Stats{cpu:self.sys.global_cpu_usage(),ram:if total>0.0 {used/total*100.0}else{0.0},
     used_ram:used/1073741824.0,total_ram:total/1073741824.0,
     disk:if dt>0 {du as f32/dt as f32*100.0}else{0.0},
     net_down:self.nets.iter().map(|(_,n)|n.received()).sum(),
     net_up:self.nets.iter().map(|(_,n)|n.transmitted()).sum(),
     cpu_name:self.sys.cpus().first().map(|c|c.brand().to_string()).unwrap_or_default(),
     os:System::long_os_version().unwrap_or_default(),uptime:System::uptime(),..Default::default()};
   if sensors_http {read_lhm(&mut s);}
   s
 }
}
// Optional LibreHardwareMonitor web server, local-only, with a hard 300ms timeout.
fn read_lhm(s:&mut Stats) {
 if let Ok(mut stream)=TcpStream::connect_timeout(&"127.0.0.1:8085".parse().unwrap(),Duration::from_millis(250)) {
  stream.set_read_timeout(Some(Duration::from_millis(300))).ok();
  if stream.write_all(b"GET /data.json HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\n\r\n").is_err(){return;}
  let mut buf=Vec::new(); let _=stream.take(262144).read_to_end(&mut buf);
  if let Some(i)=buf.windows(4).position(|x|x==b"\r\n\r\n") {
   if let Ok(v)=serde_json::from_slice::<serde_json::Value>(&buf[i+4..]) { walk(&v,"",s); }
  }
 }
}
fn walk(v:&serde_json::Value,parent:&str,s:&mut Stats) {
 let title=v["Text"].as_str().unwrap_or("");
 let parent_l=parent.to_ascii_lowercase(); let low=title.to_ascii_lowercase();
 if let Some(val)=v["Value"].as_str().and_then(|x|x.split_whitespace().next()).and_then(|x|x.replace(',','.').parse::<f32>().ok()) {
   if parent_l.contains("temperature") && (low.contains("cpu package")||low.contains("cpu core")) && s.cpu_temp.is_none(){s.cpu_temp=Some(val);}
   if parent_l.contains("temperature") && (low.contains("gpu core")||low.contains("gpu hot spot")) && s.gpu_temp.is_none(){s.gpu_temp=Some(val);}
   if parent_l.contains("load") && low.contains("gpu core") {s.gpu_load=Some(val);}
 }
 if let Some(children)=v["Children"].as_array(){for ch in children {walk(ch, title,s)}}
}
