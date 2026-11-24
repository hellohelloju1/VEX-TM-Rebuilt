use std::path::{Path,PathBuf};
use anyhow::{Context,Result};

pub fn resources() -> Result<PathBuf> {
    let exe=std::env::current_exe()?;
    #[cfg(target_os="macos")]
    {Ok(exe.parent().and_then(|p|p.parent()).context("App resources missing")?.join("Resources"))}
    #[cfg(not(target_os="macos"))]
    {Ok(exe.parent().context("App resources missing")?.to_path_buf())}
}
pub fn open(path:&Path)->Result<()> {
    #[cfg(target_os="macos")]
    let mut command=std::process::Command::new("/usr/bin/open");
    #[cfg(target_os="windows")]
    let mut command={let mut c=std::process::Command::new("explorer.exe");use std::os::windows::process::CommandExt;c.creation_flags(0x08000000);c};
    #[cfg(not(any(target_os="macos",target_os="windows")))]
    let mut command=std::process::Command::new("xdg-open");
    command.arg(path).spawn().context("Cannot open report")?;Ok(())
}
pub fn play_sound(name:&str) {
    if !["Start.wav","Pause.wav","Warning.wav","Warning2.wav","Stop.wav","Abort.wav","FiveSecondCountdown.wav"].contains(&name){return;}
    let Ok(resources)=resources() else{return};let file=resources.join("Sounds").join(name);if !file.is_file(){return;}
    #[cfg(target_os="macos")]
    if let Ok(mut child)=std::process::Command::new("/usr/bin/afplay").arg(file).spawn(){std::thread::spawn(move||{let _=child.wait();});}
    #[cfg(target_os="windows")]
    std::thread::spawn(move||{use std::os::windows::ffi::OsStrExt;#[link(name="winmm")]unsafe extern "system"{fn PlaySoundW(sound:*const u16,module:*mut std::ffi::c_void,flags:u32)->i32;}let name:Vec<u16>=file.as_os_str().encode_wide().chain(Some(0)).collect();unsafe{PlaySoundW(name.as_ptr(),std::ptr::null_mut(),0x00020002);}});
}

pub fn install_driver(name:&str)->Result<()> {
    #[cfg(target_os="windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        let relative=match name{"cp210x"=>"Drivers/cp210x/DriverInstaller_x64.exe","msvc"=>"Drivers/msvc/vcredist_x86.exe",_=>anyhow::bail!("Unknown driver installer")};
        let file=resources()?.join(relative);anyhow::ensure!(file.is_file(),"Bundled installer is missing");
        let file:Vec<u16>=file.as_os_str().encode_wide().chain(Some(0)).collect();let verb:Vec<u16>="runas".encode_utf16().chain(Some(0)).collect();
        #[link(name="shell32")]unsafe extern "system"{fn ShellExecuteW(window:*mut std::ffi::c_void,verb:*const u16,file:*const u16,parameters:*const u16,directory:*const u16,show:i32)->isize;}
        let result=unsafe{ShellExecuteW(std::ptr::null_mut(),verb.as_ptr(),file.as_ptr(),std::ptr::null(),std::ptr::null(),1)};
        anyhow::ensure!(result>32,"Installer did not start or administrator permission was declined");Ok(())
    }
    #[cfg(not(target_os="windows"))]
    {let _=name;anyhow::bail!("These driver installers are for Windows")}
}
