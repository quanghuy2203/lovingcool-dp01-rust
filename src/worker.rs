use std::{sync::mpsc::{Receiver,SyncSender},time::{Duration,Instant},thread};
use image::{RgbImage,Rgb};
use imageproc::drawing::draw_text_mut;
use ab_glyph::{FontArc,PxScale};
use serde::{Serialize,Deserialize};
use crate::{device::{Panel,candidates},sensors::{Monitor,Stats},wallpaper};
#[derive(Clone,PartialEq,Debug,Serialize,Deserialize)]
pub enum Mode {Dashboard,Image,Gif,Wallpaper}
#[derive(Clone,Serialize,Deserialize)]
pub struct Settings {pub mode:Mode,pub path:String,pub width:u32,pub height:u32,pub fps:u32,pub overlay:bool,pub sensors_http:bool,pub brightness:u8,pub streaming:bool}
impl Default for Settings {fn default()->Self{Self{mode:Mode::Dashboard,path:String::new(),width:480,height:480,fps:8,overlay:true,sensors_http:false,brightness:75,streaming:false}}}
pub enum Command {Config(Settings),Connect(String),Disconnect,Scan,Stop}
pub enum Event {Stats(Stats),Ports(Vec<String>),Connected(String,u32,u32),Disconnected(String),Frame(RgbImage),Error(String)}
fn font()->Option<FontArc>{
 [r"C:\Windows\Fonts\segoeui.ttf",r"C:\Windows\Fonts\arial.ttf","/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"].iter()
 .find_map(|p|std::fs::read(p).ok().and_then(|d|FontArc::try_from_vec(d).ok()))
}
fn label(img:&mut RgbImage,f:&Option<FontArc>,x:i32,y:i32,size:f32,text:&str,color:[u8;3]) {
 if let Some(f)=f {draw_text_mut(img,Rgb(color),x,y,PxScale::from(size),f,text);}
}
fn bar(img:&mut RgbImage,x:u32,y:u32,w:u32,value:f32,color:[u8;3]) {
 for j in y..(y+9).min(img.height()) {for i in x..(x+w).min(img.width()) {
   img.put_pixel(i,j,Rgb(if i-x < (w as f32*value.clamp(0.,100.)/100.) as u32{color}else{[32,44,61]}));
 }}
}
fn render(stats:&Stats,settings:&Settings,font:&Option<FontArc>,base:Option<&RgbImage>)->RgbImage {
 let (w,h)=(settings.width.max(64),settings.height.max(64));
 let mut img=base.map(|b|image::imageops::resize(b,w,h,image::imageops::FilterType::Triangle)).unwrap_or_else(||RgbImage::from_pixel(w,h,Rgb([11,18,33])));
 if !settings.overlay && base.is_some(){return img;}
 let scale=(w as f32/480.).max(0.5);
 // Semi-transparent glass area on the left; frame content remains visible elsewhere.
 for y in 0..h {for x in 0..(w*78/100){let p=img.get_pixel_mut(x,y);let a=if base.is_some(){0.76}else{0.97};
  for c in 0..3 {p.0[c]=(p.0[c] as f32*(1.-a)+[12u8,21,37][c] as f32*a) as u8;}
 }}
 let t=|n:i32| (n as f32*scale) as i32;
 label(&mut img,font,t(22),t(18),27.*scale,"LOVINGCOOL  /  PC",[96,228,222]);
 label(&mut img,font,t(22),t(73),21.*scale,&format!("CPU    {:>5.1}%",stats.cpu),[238,244,255]);
 bar(&mut img,t(24) as u32,t(110) as u32,t(315) as u32,stats.cpu,[68,211,205]);
 label(&mut img,font,t(22),t(134),21.*scale,&format!("RAM    {:>5.1}%",stats.ram),[238,244,255]);
 bar(&mut img,t(24) as u32,t(171) as u32,t(315) as u32,stats.ram,[109,153,254]);
 label(&mut img,font,t(22),t(199),18.*scale,&format!("CPU TEMP  {}",stats.cpu_temp.map(|v|format!("{v:.0} C")).unwrap_or_else(||"N/A".into())),[255,193,119]);
 label(&mut img,font,t(22),t(232),18.*scale,&format!("GPU TEMP  {}",stats.gpu_temp.map(|v|format!("{v:.0} C")).unwrap_or_else(||"N/A".into())),[255,193,119]);
 label(&mut img,font,t(22),t(271),18.*scale,&format!("GPU LOAD  {}",stats.gpu_load.map(|v|format!("{v:.0}%")).unwrap_or_else(||"N/A".into())),[225,234,255]);
 label(&mut img,font,t(22),t(309),18.*scale,&format!("DISK       {:.0}%",stats.disk),[225,234,255]);
 label(&mut img,font,t(22),t(346),16.*scale,&format!("RAM   {:.1} / {:.1} GB",stats.used_ram,stats.total_ram),[171,191,218]);
 label(&mut img,font,t(22),t(382),15.*scale,&format!("NET  D:{} KB/s  U:{} KB/s",stats.net_down/1024,stats.net_up/1024),[171,191,218]);
 label(&mut img,font,t(22),t(423),15.*scale,&format!("UPTIME  {}h {}m",stats.uptime/3600,stats.uptime/60%60),[171,191,218]);
 img
}
struct GifStream {
 path:String,decoder:gif::Decoder<std::fs::File>,
 canvas:RgbImage,last_disposal:gif::DisposalMethod,last_rect:(u32,u32,u32,u32),
}
impl GifStream {
 fn open(path:&str)->Result<Self,String>{
  let file=std::fs::File::open(path).map_err(|e|e.to_string())?;
  let mut opts=gif::DecodeOptions::new();opts.set_color_output(gif::ColorOutput::RGBA);
  let decoder=opts.read_info(file).map_err(|e|e.to_string())?;
  let (w,h)=(decoder.width() as u32,decoder.height() as u32);
  if w==0||h==0||w>4096||h>4096 {return Err("GIF kích thước không hợp lệ hoặc quá lớn".into());}
  Ok(Self{path:path.into(),decoder,canvas:RgbImage::from_pixel(w,h,Rgb([12,18,32])),
    last_disposal:gif::DisposalMethod::Keep,last_rect:(0,0,0,0)})
 }
 fn next(&mut self)->Result<(RgbImage,Duration),String>{
  // Decode only one frame per tick; keep one LCD-sized composite instead of all frames.
  let first=self.decoder.read_next_frame().map_err(|e|e.to_string())?.map(|f|f.clone());
  let frame=match first {
   Some(f)=>f, None=>{*self=Self::open(&self.path)?;self.decoder.read_next_frame().map_err(|e|e.to_string())?.ok_or("GIF trống")?.clone()}
  };
  if self.last_disposal==gif::DisposalMethod::Background {
   let (x,y,w,h)=self.last_rect;
   for yy in y..(y+h).min(self.canvas.height()) {for xx in x..(x+w).min(self.canvas.width()){
    self.canvas.put_pixel(xx,yy,Rgb([12,18,32]));
   }}
  }
  let (left,top)=(frame.left as u32,frame.top as u32);
  for y in 0..frame.height as u32 {for x in 0..frame.width as u32 {
   let idx=(y*frame.width as u32+x) as usize*4;
   let xx=x+left;let yy=y+top;
   if xx<self.canvas.width()&&yy<self.canvas.height()&&idx+3<frame.buffer.len()&&frame.buffer[idx+3]>128 {
    self.canvas.put_pixel(xx,yy,Rgb([frame.buffer[idx],frame.buffer[idx+1],frame.buffer[idx+2]]));
   }
  }}
  self.last_rect=(left,top,frame.width as u32,frame.height as u32);
  self.last_disposal=frame.dispose;
  Ok((image::imageops::resize(&self.canvas,480,480,image::imageops::FilterType::Triangle),
      Duration::from_millis((frame.delay as u64*10).clamp(50,1000))))
 }
}
fn media(settings:&Settings,gif:&mut Option<GifStream>,next_gif:&mut Instant,last:&mut Option<RgbImage>)->Option<RgbImage>{
 match settings.mode {
 Mode::Dashboard=>None,
 Mode::Image=>{
  if last.is_none(){*last=image::open(&settings.path).ok().map(|i|i.to_rgb8());}
  last.as_ref().map(|i|image::imageops::resize(i,settings.width,settings.height,image::imageops::FilterType::Triangle))
 },
 Mode::Gif=>{
  if gif.is_none(){*gif=GifStream::open(&settings.path).ok();}
  if Instant::now()>=*next_gif {
   if let Some(g)=gif {if let Ok((f,d))=g.next(){*last=Some(f);*next_gif=Instant::now()+d;}}
  }
  last.as_ref().map(|i|image::imageops::resize(i,settings.width,settings.height,image::imageops::FilterType::Triangle))
 },
 Mode::Wallpaper=>wallpaper::capture(settings.width,settings.height).ok(),
 }
}
pub fn start(rx:Receiver<Command>,tx:SyncSender<Event>){
 thread::spawn(move||{
  let mut monitor=Monitor::new();let f=font();let mut config=Settings::default();let mut panel:Option<Panel>=None;
  let mut gif=None;let mut next_gif=Instant::now();let mut last_media=None;
  let mut next_sensor=Instant::now();let mut next_render=Instant::now();let mut last_keepalive=Instant::now();
  let mut stats=Stats::default();let mut dirty=true;let _=tx.send(Event::Ports(candidates()));
  loop {
   while let Ok(cmd)=rx.try_recv(){match cmd {
    Command::Stop=>return,
    Command::Scan=>{let _=tx.send(Event::Ports(candidates()));},
    Command::Disconnect=>{panel=None;let _=tx.send(Event::Disconnected("Đã ngắt kết nối".into()));},
    Command::Connect(port)=>{
     panel=None;match Panel::connect(&port){Ok(mut p)=>{let _=p.brightness(config.brightness);let _=p.keepalive();
      let _=tx.send(Event::Connected(format!("{} / {} / {}",port,p.model,p.firmware),p.width,p.height));panel=Some(p);
     },Err(e)=>{let _=tx.send(Event::Error(e));}}
    },
    Command::Config(next)=>{
     if config.path!=next.path||config.mode!=next.mode{gif=None;last_media=None;next_gif=Instant::now();}
     if config.brightness!=next.brightness{if let Some(p)=&mut panel {let _=p.brightness(next.brightness);}}
     config=next;dirty=true;next_render=Instant::now();
    }
   }}
   let now=Instant::now();
   if now>=next_sensor {stats=monitor.sample(config.sensors_http);let _=tx.send(Event::Stats(stats.clone()));next_sensor=now+Duration::from_secs(2);dirty=true;}
   if let Some(p)=&mut panel {if now.duration_since(last_keepalive)>=Duration::from_millis(1200) {
    if let Err(e)=p.keepalive(){let _=tx.send(Event::Error(e));panel=None;}last_keepalive=now;
   }}
   if now>=next_render {
    let animated=matches!(config.mode,Mode::Gif|Mode::Wallpaper);
    if dirty||animated {
     let bg=media(&config,&mut gif,&mut next_gif,&mut last_media);
     let frame=render(&stats,&config,&f,bg.as_ref());
     if config.streaming {if let Some(p)=&mut panel {if let Err(e)=p.send(&frame){let _=tx.send(Event::Error(e));panel=None;}}}
     // UI preview throttled separately from USB send, avoiding a growing queue.
     if dirty||now.duration_since(next_render)<Duration::from_millis(120) {let _=tx.try_send(Event::Frame(frame));}
     dirty=false;
    }
    next_render=now+if animated {Duration::from_millis(1000/config.fps.clamp(1,15) as u64)}else{Duration::from_secs(2)};
   }
   thread::sleep(Duration::from_millis(if matches!(config.mode,Mode::Gif|Mode::Wallpaper){20}else{160}));
  }
 });
}
