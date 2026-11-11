#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod device;
mod updates;
mod sensors;
mod wallpaper;
mod worker;
use std::sync::mpsc::{channel,sync_channel,Sender,Receiver};
use eframe::egui::{self,Color32,RichText,TextureHandle};
use worker::{Settings,Mode,Command,Event};

struct Studio {
 tx:Sender<Command>,rx:Receiver<Event>,settings:Settings,
 ports:Vec<String>,selected:String,status:String,stats:sensors::Stats,
 texture:Option<TextureHandle>,connected:bool,
 wallpaper_exe:String,wallpaper_project:String,update_repo:String,
}
impl Studio {
 fn new(cc:&eframe::CreationContext<'_>)->Self {
  let mut style=(*cc.egui_ctx.style()).clone();
  style.visuals=egui::Visuals::dark();
  style.visuals.panel_fill=Color32::from_rgb(12,19,32);
  style.visuals.widgets.active.bg_fill=Color32::from_rgb(34,125,133);
  style.visuals.selection.bg_fill=Color32::from_rgb(35,123,133);
  cc.egui_ctx.set_style(style);
  let (tx,rx_worker)=channel();let (tx_event,rx)=sync_channel(2);worker::start(rx_worker,tx_event);
  let defaults=updates::load_settings().unwrap_or_default();let _=tx.send(Command::Config(defaults.clone()));
  Self{tx,rx,settings:defaults,ports:vec![],selected:String::new(),status:"Chưa kết nối LCD — preview vẫn hoạt động".into(),stats:Default::default(),texture:None,connected:false,wallpaper_exe:r"C:\Program Files (x86)\Steam\steamapps\common\wallpaper_engine\wallpaper64.exe".into(),wallpaper_project:String::new(),update_repo:"quanghuy2203/lovingcool-dp01-rust".into()}
 }
 fn changed(&self){let _=self.tx.send(Command::Config(self.settings.clone()));updates::save_settings(&self.settings);}
 fn metric(ui:&mut egui::Ui,name:&str,val:String,percent:Option<f32>){
  ui.group(|ui|{ui.set_min_width(170.);ui.label(RichText::new(name).color(Color32::from_rgb(147,174,199)).size(13.));
   ui.label(RichText::new(val).size(23.).strong().color(Color32::WHITE));
   if let Some(p)=percent {ui.add(egui::ProgressBar::new(p.clamp(0.,100.)/100.).fill(Color32::from_rgb(45,202,195)));}
  });
 }
}
impl eframe::App for Studio {
 fn update(&mut self,ctx:&egui::Context,_frame:&mut eframe::Frame){
  if updates::update_is_ready(){ctx.send_viewport_cmd(egui::ViewportCommand::Close);return;}
  // Drain all events, keeping only newest preview frame. Do not buffer decoded frames in RAM.
  let mut latest=None;
  while let Ok(ev)=self.rx.try_recv(){match ev{
   Event::Stats(s)=>self.stats=s, Event::Ports(p)=>{self.ports=p;if self.selected.is_empty(){self.selected=self.ports.first().cloned().unwrap_or_default();}},
   Event::Connected(s,w,h)=>{self.connected=true;self.status=format!("Đã kết nối: {s} ({w}x{h})");self.settings.width=w;self.settings.height=h;self.settings.streaming=true;self.changed();},
   Event::Disconnected(s)=>{self.connected=false;self.settings.streaming=false;self.status=s;self.changed();},
   Event::Error(s)=>{self.status=format!("Lỗi: {s}");self.connected=false;self.settings.streaming=false;self.changed();},
   Event::Frame(f)=>latest=Some(f),
  }}
  if let Some(f)=latest {
   let pixels=f.pixels().map(|p|Color32::from_rgb(p[0],p[1],p[2])).collect();
   let img=egui::ColorImage{size:[f.width() as usize,f.height() as usize],pixels};
   if let Some(tex)=&mut self.texture{tex.set(img,egui::TextureOptions::LINEAR)} else{self.texture=Some(ctx.load_texture("lcd",img,egui::TextureOptions::LINEAR));}
  }
  egui::TopBottomPanel::top("header").show(ctx,|ui|{
   ui.add_space(9.);ui.horizontal(|ui|{
    ui.label(RichText::new("◈  LOVINGCOOL STUDIO").size(22.).strong().color(Color32::from_rgb(72,222,215)));
    ui.separator();ui.label("DP-01  •  Rust  •  Windows");
   });ui.add_space(7.);
  });
  egui::SidePanel::left("controls").min_width(300.).max_width(355.).show(ctx,|ui|{
   egui::ScrollArea::vertical().show(ui,|ui|{
   ui.add_space(12.);ui.heading("Kết nối LCD (không có RGB/LED)");
   ui.horizontal(|ui|{
    egui::ComboBox::from_id_salt("port").selected_text(if self.selected.is_empty(){"Chưa thấy thiết bị"}else{&self.selected})
      .show_ui(ui,|ui|{for port in &self.ports {ui.selectable_value(&mut self.selected,port.clone(),port);}});
    if ui.button("Quét lại").clicked(){let _=self.tx.send(Command::Scan);}
   });
   ui.horizontal(|ui|{
    if ui.add_enabled(!self.connected&&!self.selected.is_empty(),egui::Button::new("Kết nối")).clicked(){let _=self.tx.send(Command::Connect(self.selected.clone()));}
    if ui.add_enabled(self.connected,egui::Button::new("Ngắt kết nối")).clicked(){let _=self.tx.send(Command::Disconnect);}
   });
   ui.label(RichText::new(&self.status).small().color(Color32::from_rgb(171,193,212)));
   ui.separator();ui.heading("Nội dung LCD");
   let mut ch=false;
   ch|=ui.radio_value(&mut self.settings.mode,Mode::Dashboard,"Dashboard PC").changed();
   ch|=ui.radio_value(&mut self.settings.mode,Mode::Image,"Ảnh JPG / PNG").changed();
   ch|=ui.radio_value(&mut self.settings.mode,Mode::Gif,"GIF (đọc từng frame)").changed();
   ch|=ui.radio_value(&mut self.settings.mode,Mode::Wallpaper,"Wallpaper Engine (thử nghiệm)").changed();
   if matches!(self.settings.mode,Mode::Image|Mode::Gif){ui.label("Đường dẫn tệp ảnh:");ch|=ui.text_edit_singleline(&mut self.settings.path).changed();}
   if self.settings.mode==Mode::Wallpaper {
    ui.label("wallpaper64.exe:");ui.text_edit_singleline(&mut self.wallpaper_exe);
    ui.label("Project (project.json / index.html / scene.pkg):");ui.text_edit_singleline(&mut self.wallpaper_project);
    if ui.button("Mở wallpaper trong cửa sổ riêng").clicked(){match wallpaper::launch(&self.wallpaper_exe,&self.wallpaper_project,self.settings.width,self.settings.height){Ok(())=>self.status="Đã gửi lệnh mở Wallpaper Engine; đang chờ cửa sổ render".into(),Err(e)=>self.status=e}}
    ui.label(RichText::new("Chụp GDI có thể trả về khung hình đen đối với nội dung GPU/WebGL. Scene và Web cần Wallpaper Engine đang chạy.").small().color(Color32::YELLOW));
   }
   ch|=ui.checkbox(&mut self.settings.overlay,"Đè thông số PC lên hình nền").changed();
   ch|=ui.checkbox(&mut self.settings.sensors_http,"Đọc cảm biến từ LibreHardwareMonitor :8085").changed();
   ui.label(format!("FPS ảnh động: {}",self.settings.fps));ch|=ui.add(egui::Slider::new(&mut self.settings.fps,1..=15)).changed();
   ui.label(format!("Độ sáng LCD: {}%",self.settings.brightness));ch|=ui.add(egui::Slider::new(&mut self.settings.brightness,1..=100)).changed();
   if ch {self.changed();}
   ui.separator();ui.heading("Tiết kiệm tài nguyên");
   ui.label("• Đọc sensor mỗi 2 giây");ui.label("• Dashboard tĩnh không render liên tục");
   ui.label("• GIF giải mã từng khung hình");ui.label("• Giới hạn ảnh động tối đa 15 FPS");
   ui.label("• Không nhúng trình duyệt vào tiến trình Rust");
   ui.separator();ui.heading("Cập nhật từ GitHub Releases");
   ui.label("Repository (owner/repo):");ui.text_edit_singleline(&mut self.update_repo);
   if ui.button("Kiểm tra và cài bản mới").clicked(){
    match updates::check_and_update(&self.update_repo,false){Ok(())=>self.status="Đã mở trình kiểm tra cập nhật".into(),Err(e)=>self.status=e}
   }
   ui.label(RichText::new("Bản mới chỉ được cài nếu có Release, gói ZIP và checksum SHA-256 hợp lệ.").small());
   ui.add_space(20.);
   });
  });
  egui::CentralPanel::default().show(ctx,|ui|{
   ui.add_space(12.);ui.heading("Live overview");ui.label(RichText::new(&self.stats.cpu_name).color(Color32::from_rgb(174,196,216)));
   ui.add_space(8.);
   ui.horizontal_wrapped(|ui|{
    Self::metric(ui,"CPU",format!("{:.1}%",self.stats.cpu),Some(self.stats.cpu));
    Self::metric(ui,"RAM",format!("{:.1} / {:.1} GB",self.stats.used_ram,self.stats.total_ram),Some(self.stats.ram));
    Self::metric(ui,"Disk",format!("{:.0}%",self.stats.disk),Some(self.stats.disk));
    Self::metric(ui,"CPU temp",self.stats.cpu_temp.map(|x|format!("{x:.0} °C")).unwrap_or("N/A".into()),None);
    Self::metric(ui,"GPU temp",self.stats.gpu_temp.map(|x|format!("{x:.0} °C")).unwrap_or("N/A".into()),None);
    Self::metric(ui,"GPU load",self.stats.gpu_load.map(|x|format!("{x:.0}%")).unwrap_or("N/A".into()),self.stats.gpu_load);
   });
   ui.add_space(13.);ui.heading("Màn hình DP-01 (preview)");
   if let Some(t)=&self.texture{let avail=ui.available_size();let aspect=t.size()[0] as f32/t.size()[1] as f32;
    let width=avail.x.min(450.).min(avail.y*aspect).max(80.);ui.image((t.id(),egui::vec2(width,width/aspect)));
   }else{ui.spinner();ui.label("Đang tạo khung hình đầu tiên");}
   ui.add_space(7.);ui.label(format!("Network  ↓ {} KB/s  ↑ {} KB/s   |   Uptime {}h {}m",self.stats.net_down/1024,self.stats.net_up/1024,self.stats.uptime/3600,self.stats.uptime/60%60));
  });
  // UI refresh infrequently; active animations are driven by the background worker.
  ctx.request_repaint_after(Duration::from_millis(if matches!(self.settings.mode,Mode::Gif|Mode::Wallpaper){200}else{1000}));
 }
}
fn main()->eframe::Result<()> {
 // One short-lived update check after startup; no resident polling process.
 let _=updates::check_and_update("quanghuy2203/lovingcool-dp01-rust",true);
 let opts=eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_inner_size([1100.,740.]).with_min_inner_size([790.,560.]),..Default::default()};
 eframe::run_native("LovingCool DP-01 Studio",opts,Box::new(|cc|Ok(Box::new(Studio::new(cc)))))
}
