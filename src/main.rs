#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod device;
mod updates;
mod sensors;
mod wallpaper;
mod worker;
use std::sync::mpsc::{channel,sync_channel,Sender,Receiver};
use std::time::Duration;
use eframe::egui::{self,Color32,RichText,TextureHandle,FontFamily,FontData,FontDefinitions,FontId};
use worker::{Settings,Mode,Command,Event};

struct Studio {
 tx:Sender<Command>,rx:Receiver<Event>,settings:Settings,
 ports:Vec<String>,selected:String,status:String,stats:sensors::Stats,
 texture:Option<TextureHandle>,connected:bool,
 wallpaper_exe:String,wallpaper_project:String,
}
impl Studio {
 fn new(cc:&eframe::CreationContext<'_>)->Self {
  // egui's bundled font lacks Vietnamese glyphs; load a native Windows font.
  let mut fonts=FontDefinitions::default();
  let font_paths=[r"C:\Windows\Fonts\segoeui.ttf",r"C:\Windows\Fonts\arial.ttf"];
  if let Some(bytes)=font_paths.iter().find_map(|p|std::fs::read(p).ok()){
   fonts.font_data.insert("vietnamese".into(),std::sync::Arc::new(FontData::from_owned(bytes)));
   fonts.families.entry(FontFamily::Proportional).or_default().insert(0,"vietnamese".into());
  }
  cc.egui_ctx.set_fonts(fonts);
  let mut style=(*cc.egui_ctx.style()).clone();
  style.visuals=egui::Visuals::dark();
  style.visuals.window_fill=Color32::from_rgb(11,17,29);
  style.visuals.panel_fill=Color32::from_rgb(11,17,29);
  style.visuals.extreme_bg_color=Color32::from_rgb(16,25,40);
  style.visuals.faint_bg_color=Color32::from_rgb(20,31,49);
  style.visuals.selection.bg_fill=Color32::from_rgb(37,141,156);
  style.visuals.widgets.inactive.bg_fill=Color32::from_rgb(26,39,58);
  style.visuals.widgets.hovered.bg_fill=Color32::from_rgb(40,63,82);
  style.visuals.widgets.active.bg_fill=Color32::from_rgb(31,135,150);
  style.spacing.item_spacing=egui::vec2(10.,10.);
  style.spacing.button_padding=egui::vec2(13.,8.);
  style.spacing.interact_size.y=30.;
  cc.egui_ctx.set_style(style);
  let (tx,rx_worker)=channel();let (tx_event,rx)=sync_channel(2);worker::start(rx_worker,tx_event);
  let defaults=updates::load_settings().unwrap_or_default();let _=tx.send(Command::Config(defaults.clone()));
  Self{tx,rx,settings:defaults,ports:vec![],selected:String::new(),status:"Chưa kết nối LCD — preview vẫn hoạt động".into(),stats:Default::default(),texture:None,connected:false,wallpaper_exe:r"C:\Program Files (x86)\Steam\steamapps\common\wallpaper_engine\wallpaper64.exe".into(),wallpaper_project:String::new()}
 }
 fn changed(&self){let _=self.tx.send(Command::Config(self.settings.clone()));updates::save_settings(&self.settings);}
 fn metric(ui:&mut egui::Ui,name:&str,val:String,percent:Option<f32>,accent:Color32){
  egui::Frame::new().fill(Color32::from_rgb(20,31,48))
   .stroke(egui::Stroke::new(1.,Color32::from_rgb(37,53,74))).corner_radius(12.)
   .inner_margin(egui::Margin::same(16)).show(ui,|ui|{
    ui.set_min_width(175.);
    ui.set_min_height(102.);
    ui.label(RichText::new(name).size(13.).color(Color32::from_rgb(156,178,204)));
    ui.add_space(5.);
    ui.label(RichText::new(val).size(24.).strong().color(Color32::from_rgb(239,246,255)));
    ui.add_space(9.);
    if let Some(p)=percent{
     ui.add(egui::ProgressBar::new((p/100.).clamp(0.,1.)).fill(accent).desired_height(6.).show_percentage());
    }else{ui.label(RichText::new("Cần bật nguồn cảm biến").size(11.).color(Color32::from_rgb(119,141,165)));}
   });
 }
 fn section(ui:&mut egui::Ui,title:&str,description:&str){
  ui.label(RichText::new(title).size(17.).strong().color(Color32::from_rgb(241,247,255)));
  ui.label(RichText::new(description).size(12.).color(Color32::from_rgb(143,167,192)));
  ui.add_space(5.);
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
  egui::TopBottomPanel::top("header").exact_height(72.).show(ctx,|ui|{
   ui.add_space(10.);
   ui.horizontal(|ui|{
    ui.add_space(14.);
    ui.label(RichText::new("◈").size(29.).color(Color32::from_rgb(70,223,212)));
    ui.vertical(|ui|{
     ui.label(RichText::new("LOVINGCOOL").size(20.).strong().color(Color32::from_rgb(232,245,250)));
     ui.label(RichText::new("DP-01 STUDIO  /  WINDOWS").size(10.).color(Color32::from_rgb(119,155,178)));
    });
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
     ui.add_space(15.);
     ui.label(RichText::new(format!("v{}  •  Rust",env!("CARGO_PKG_VERSION"))).size(12.).color(Color32::from_rgb(143,176,196)));
    });
   });
  });
  egui::SidePanel::left("controls").resizable(true).default_width(320.).min_width(290.).max_width(430.).show(ctx,|ui|{
   egui::ScrollArea::vertical().auto_shrink([false,false]).show(ui,|ui|{
    ui.add_space(13.);
    Self::section(ui,"THIẾT BỊ","Màn hình LCD USB • không điều khiển RGB/LED");
    egui::ComboBox::from_id_salt("port").width(ui.available_width())
     .selected_text(if self.selected.is_empty(){"Chưa tìm thấy LCD"}else{&self.selected})
     .show_ui(ui,|ui|{for port in &self.ports{ui.selectable_value(&mut self.selected,port.clone(),port);}});
    ui.horizontal(|ui|{
     if ui.button("⟳ Quét").clicked(){let _=self.tx.send(Command::Scan);}
     if ui.add_enabled(!self.connected&&!self.selected.is_empty(),egui::Button::new("Kết nối LCD")).clicked(){
      let _=self.tx.send(Command::Connect(self.selected.clone()));
     }
     if ui.add_enabled(self.connected,egui::Button::new("Ngắt")).clicked(){
      let _=self.tx.send(Command::Disconnect);
     }
    });
    ui.label(RichText::new(&self.status).size(12.).color(Color32::from_rgb(142,179,192)));
    ui.add_space(14.);ui.separator();ui.add_space(8.);
    Self::section(ui,"NỘI DUNG HIỂN THỊ","Chọn nguồn hình và lớp thông số");
    let mut ch=false;
    ch|=ui.radio_value(&mut self.settings.mode,Mode::Dashboard,"Dashboard PC").changed();
    ch|=ui.radio_value(&mut self.settings.mode,Mode::Image,"Ảnh PNG / JPG").changed();
    ch|=ui.radio_value(&mut self.settings.mode,Mode::Gif,"Ảnh động GIF").changed();
    ch|=ui.radio_value(&mut self.settings.mode,Mode::Wallpaper,"Wallpaper Engine • Web / Scene").changed();
    if matches!(self.settings.mode,Mode::Image|Mode::Gif){
     ui.label("Đường dẫn tệp ảnh");
     ch|=ui.add(egui::TextEdit::singleline(&mut self.settings.path).desired_width(f32::INFINITY)).changed();
    }
    if self.settings.mode==Mode::Wallpaper{
     ui.label("Đường dẫn wallpaper64.exe");
     ui.add(egui::TextEdit::singleline(&mut self.wallpaper_exe).desired_width(f32::INFINITY));
     ui.label("Project Wallpaper Engine");
     ui.add(egui::TextEdit::singleline(&mut self.wallpaper_project).desired_width(f32::INFINITY));
     if ui.button("Mở wallpaper").clicked(){
      match wallpaper::launch(&self.wallpaper_exe,&self.wallpaper_project,self.settings.width,self.settings.height){
       Ok(())=>self.status="Đã gửi lệnh mở Wallpaper Engine".into(),Err(e)=>self.status=e
      }
     }
     ui.label(RichText::new("Chế độ thử nghiệm: một số wallpaper GPU/WebGL có thể hiện màn hình đen.").size(11.).color(Color32::from_rgb(239,179,104)));
    }
    ui.add_space(6.);
    ch|=ui.checkbox(&mut self.settings.overlay,"Hiển thị thông số PC lên hình").changed();
    ch|=ui.checkbox(&mut self.settings.sensors_http,"Đọc nhiệt độ từ LibreHardwareMonitor").changed();
    ui.add_space(5.);
    ui.label(format!("FPS ảnh động  •  {}",self.settings.fps));
    ch|=ui.add(egui::Slider::new(&mut self.settings.fps,1..=15).show_value(false)).changed();
    ui.label(format!("Độ sáng LCD  •  {}%",self.settings.brightness));
    ch|=ui.add(egui::Slider::new(&mut self.settings.brightness,1..=100).show_value(false)).changed();
    if ch{self.changed();}
    ui.add_space(12.);ui.separator();ui.add_space(8.);
    Self::section(ui,"HIỆU NĂNG","Ưu tiên tiết kiệm RAM khi chạy nền");
    ui.label(RichText::new("• Cảm biến: cập nhật mỗi 2 giây\n• Dashboard: không render liên tục\n• GIF: giải mã từng frame\n• Không nhúng trình duyệt trong app").size(12.).color(Color32::from_rgb(167,191,211)));
    ui.add_space(12.);ui.separator();ui.add_space(8.);
    Self::section(ui,"CẬP NHẬT","Nhận bản mới trực tiếp từ GitHub");
    ui.label(format!("Phiên bản v{}",env!("CARGO_PKG_VERSION")));
    if ui.button("↻ Kiểm tra, tải và cài cập nhật").clicked(){
     match updates::check_and_update("quanghuy2203/lovingcool-dp01-rust",false){
      Ok(())=>self.status="Đang kiểm tra phiên bản mới…".into(),Err(e)=>self.status=e
     }
    }
    if let Some(progress)=updates::update_status(){
     ui.label(RichText::new(progress).size(12.).color(Color32::from_rgb(103,220,198)));
    }
    ui.add_space(20.);
   });
  });
  egui::CentralPanel::default().show(ctx,|ui|{
   egui::ScrollArea::vertical().show(ui,|ui|{
    ui.add_space(16.);
    ui.horizontal(|ui|{
     ui.vertical(|ui|{
      ui.label(RichText::new("Tổng quan hệ thống").size(25.).strong().color(Color32::from_rgb(237,247,255)));
      ui.label(RichText::new(&self.stats.cpu_name).size(13.).color(Color32::from_rgb(144,171,198)));
     });
    });
    ui.add_space(13.);
    let teal=Color32::from_rgb(62,211,203);
    let blue=Color32::from_rgb(95,147,245);
    let amber=Color32::from_rgb(239,169,91);
    let gray=Color32::from_rgb(145,164,196);
    ui.columns(3,|cols|{
     Self::metric(&mut cols[0],"CPU",format!("{:.1}%",self.stats.cpu),Some(self.stats.cpu),teal);
     Self::metric(&mut cols[1],"RAM",format!("{:.1}%",self.stats.ram),Some(self.stats.ram),blue);
     Self::metric(&mut cols[2],"Ổ đĩa",format!("{:.0}%",self.stats.disk),Some(self.stats.disk),gray);
    });
    ui.add_space(8.);
    ui.columns(3,|cols|{
     Self::metric(&mut cols[0],"Nhiệt độ CPU",self.stats.cpu_temp.map(|v|format!("{v:.0} °C")).unwrap_or("N/A".into()),None,amber);
     Self::metric(&mut cols[1],"Nhiệt độ GPU",self.stats.gpu_temp.map(|v|format!("{v:.0} °C")).unwrap_or("N/A".into()),None,amber);
     Self::metric(&mut cols[2],"GPU Load",self.stats.gpu_load.map(|v|format!("{v:.0}%")).unwrap_or("N/A".into()),self.stats.gpu_load,teal);
    });
    ui.add_space(19.);
    ui.horizontal(|ui|{
     ui.label(RichText::new("Xem trước màn hình LCD").size(19.).strong().color(Color32::from_rgb(232,241,254)));
     ui.label(RichText::new(format!("{} × {}",self.settings.width,self.settings.height)).size(12.).color(Color32::from_rgb(142,166,190)));
    });
    ui.add_space(8.);
    egui::Frame::new().fill(Color32::from_rgb(16,25,40))
      .stroke(egui::Stroke::new(1.,Color32::from_rgb(37,53,74))).corner_radius(14.)
      .inner_margin(egui::Margin::same(16)).show(ui,|ui|{
      ui.vertical_centered(|ui|{
       if let Some(t)=&self.texture{
        let available=ui.available_width();
        let aspect=t.size()[0] as f32/t.size()[1] as f32;
        let width=available.min(430.);
        ui.image((t.id(),egui::vec2(width,width/aspect)));
       }else{ui.spinner();ui.label("Đang khởi tạo preview...");}
      });
    });
    ui.add_space(12.);
    ui.label(RichText::new(format!(
      "RAM {:.1} / {:.1} GB    •    Tải xuống {} KB/s    •    Tải lên {} KB/s    •    Uptime {}h {}m",
      self.stats.used_ram,self.stats.total_ram,self.stats.net_down/1024,self.stats.net_up/1024,self.stats.uptime/3600,self.stats.uptime/60%60
    )).size(12.).color(Color32::from_rgb(154,179,203)));
    ui.add_space(15.);
   });
  });
  // UI refresh infrequently; active animations are driven by the background worker.
  ctx.request_repaint_after(Duration::from_millis(if matches!(self.settings.mode,Mode::Gif|Mode::Wallpaper){200}else{1000}));
 }
}
fn main()->eframe::Result<()> {
 let opts=eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_inner_size([1100.,740.]).with_min_inner_size([790.,560.]),..Default::default()};
 eframe::run_native("LovingCool DP-01 Studio",opts,Box::new(|cc|Ok(Box::new(Studio::new(cc)))))
}
