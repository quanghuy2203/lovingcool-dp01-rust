//! Run an on-demand PowerShell update helper, never a persistent network client.
use crate::worker::Settings;
use std::{fs,path::PathBuf,process::{Command,Stdio}};
fn config_file()->Option<PathBuf>{std::env::var_os("LOCALAPPDATA").map(|x|PathBuf::from(x).join("LovingCoolRust").join("settings.json"))}
pub fn load_settings()->Option<Settings>{serde_json::from_slice(&fs::read(config_file()?).ok()?).ok()}
pub fn save_settings(settings:&Settings){if let Some(path)=config_file(){if let Some(parent)=path.parent(){let _=fs::create_dir_all(parent);}if let Ok(data)=serde_json::to_vec_pretty(settings){let _=fs::write(path,data);}}}
pub fn check_and_update(repo:&str,silent:bool)->Result<(),String>{
 if !repo.split('/').all(|s|!s.is_empty() && s.chars().all(|c|c.is_ascii_alphanumeric()||"-_.".contains(c))) || repo.matches('/').count()!=1 {
  return Err("GitHub repo phải có dạng owner/repo".into());
 }
 let script=std::env::current_exe().map_err(|e|e.to_string())?.with_file_name("update.ps1");
 if !script.exists(){return if silent {Ok(())}else{Err("Thiếu update.ps1 cạnh EXE; bản release có kèm file này".into())};}
 let mut c=Command::new("powershell.exe");
 c.args(["-NoProfile","-ExecutionPolicy","Bypass","-File"]).arg(&script)
  .args(["-Repository",repo,"-CurrentVersion",env!("CARGO_PKG_VERSION"),"-ParentPid",&std::process::id().to_string()]);
 if silent{c.arg("-Silent");c.stdout(Stdio::null()).stderr(Stdio::null());}
 c.spawn().map_err(|e|e.to_string())?;Ok(())
}

pub fn update_is_ready()->bool{
 if let Some(path)=config_file().and_then(|p|p.parent().map(|x|x.join("update.ready"))){
  if path.exists(){let _=fs::remove_file(path);return true;}
 }
 false
}
