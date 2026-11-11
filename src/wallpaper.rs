//! Wallpaper Engine command line bridge. Capture from GPU windows is best effort.
use std::{path::Path,process::Command};
pub const WINDOW_TITLE:&str="LovingCool DP01 Wallpaper";
pub fn launch(exe:&str,project:&str,width:u32,height:u32)->Result<(),String> {
 if !Path::new(exe).exists() {return Err("Không tìm thấy wallpaper64.exe".into());}
 if !Path::new(project).exists(){return Err("Không tìm thấy project.json / index.html / scene.pkg".into());}
 Command::new(exe).args(["-control","openWallpaper","-file",project,"-playInWindow",WINDOW_TITLE,"-width",&width.to_string(),"-height",&height.to_string()])
 .spawn().map_err(|e|e.to_string())?;
 Ok(())
}
#[cfg(windows)]
pub fn capture(width:u32,height:u32)->Result<image::RgbImage,String>{
 use windows_sys::Win32::{Graphics::Gdi::*,UI::WindowsAndMessaging::*};
 use std::ptr::null;
 let title:Vec<u16>=WINDOW_TITLE.encode_utf16().chain(std::iter::once(0)).collect();
 unsafe {
  let hwnd=FindWindowW(null(),title.as_ptr());
  if hwnd.is_null(){return Err("Chưa tìm thấy cửa sổ Wallpaper Engine.".into());}
  let src=GetWindowDC(hwnd); if src.is_null(){return Err("GetWindowDC thất bại".into());}
  let mem=CreateCompatibleDC(src); let bmp=CreateCompatibleBitmap(src,width as i32,height as i32);
  if mem.is_null()||bmp.is_null(){if !bmp.is_null(){DeleteObject(bmp as _);}if !mem.is_null(){DeleteDC(mem);}ReleaseDC(hwnd,src);return Err("GDI allocation thất bại".into());}
  let old=SelectObject(mem,bmp as _);
  let _=BitBlt(mem,0,0,width as i32,height as i32,src,0,0,SRCCOPY);
  let mut bi:BITMAPINFO=std::mem::zeroed();
  bi.bmiHeader.biSize=std::mem::size_of::<BITMAPINFOHEADER>() as u32;
  bi.bmiHeader.biWidth=width as i32; bi.bmiHeader.biHeight=-(height as i32);
  bi.bmiHeader.biPlanes=1;bi.bmiHeader.biBitCount=32;bi.bmiHeader.biCompression=BI_RGB;
  let mut pixels=vec![0u8;width as usize*height as usize*4];
  let ok=GetDIBits(mem,bmp,0,height,pixels.as_mut_ptr() as *mut _,&mut bi,DIB_RGB_COLORS);
  SelectObject(mem,old);DeleteObject(bmp as _);DeleteDC(mem);ReleaseDC(hwnd,src);
  if ok==0{return Err("GetDIBits thất bại".into());}
  let mut img=image::RgbImage::new(width,height);
  for (i,p) in img.pixels_mut().enumerate(){let j=i*4;*p=image::Rgb([pixels[j+2],pixels[j+1],pixels[j]]);}
  Ok(img)
 }
}
#[cfg(not(windows))]
pub fn capture(_:u32,_:u32)->Result<image::RgbImage,String>{Err("Chỉ hỗ trợ Windows".into())}
