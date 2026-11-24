//! Uses the installed original V5 driver in an isolated Intel helper on macOS.
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::Duration,
};
pub const DEFAULT_DRIVER: &str =
    "/Applications/VEX Tournament Manager.app/Contents/MacOS/v5field-darwin-amd64.so";
pub struct V5 {
    child: Child,
    input: ChildStdin,
    replies: mpsc::Receiver<String>,
    pub connected: bool,
    pub count: usize,
}
impl V5 {
    pub fn helper() -> Result<PathBuf> {

        if let Some(path) = std::env::var_os("TMRS_V5_HELPER") {
            let p = PathBuf::from(path);
            if p.exists() {
                return Ok(p);
            }
        }
        #[cfg(target_os="windows")]
        {let p=crate::platform::resources()?.join("v5-bridge.exe");if !p.is_file(){bail!("V5 helper is missing; run the packaged Windows app")}return Ok(p);}
        let exe = std::env::current_exe()?;
        let p = exe
            .parent()
            .context("App executable folder")?
            .parent()
            .context("App Contents folder")?
            .join("Resources/v5-bridge");
        if !p.exists() {
            bail!("V5 helper is missing; run the packaged macOS app")
        };
        Ok(p)
    }
    pub fn discover(driver: &Path) -> Result<Self> {
        Self::spawn(&Self::helper()?, driver)
    }
    pub fn spawn(helper: &Path, driver: &Path) -> Result<Self> {
        let mut command=Command::new(helper);
        #[cfg(target_os="windows")]
        {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
        let mut child = command.arg(driver)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Cannot start V5 helper (Rosetta is required on Apple Silicon)")?;
        let input = child.stdin.take().context("Driver input")?;
        let output = child.stdout.take().context("Driver output")?;
        let (tx, replies) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines().map_while(Result::ok) {
                if let Some(message) = line.strip_prefix("TMRS ") {
                    if tx.send(message.into()).is_err() {
                        break;
                    }
                }
            }
        });
        let mut v = Self {
            child,
            input,
            replies,
            connected: false,
            count: 0,
        };
        let reply = v.request("count")?;
        v.count = reply["count"].as_u64().unwrap_or(0) as usize;
        Ok(v)
    }
    pub fn request(&mut self, command: &str) -> Result<Value> {
        writeln!(self.input, "{command}")?;
        self.input.flush()?;
        let line = match self.replies.recv_timeout(Duration::from_secs(3)) {
            Ok(v) => v,
            Err(e) => {
                self.connected = false;
                let _ = self.child.kill();
                bail!("V5 driver stopped responding: {e}")
            }
        };
        let v: Value = serde_json::from_str(&line).context("Invalid V5 driver reply")?;
        if v["ok"] != true {
            bail!("{}", v["error"].as_str().unwrap_or("V5 driver error"))
        };
        Ok(v)
    }
    pub fn connect(&mut self, index: usize) -> Result<()> {
        self.request(&format!("open {index}"))?;
        self.request("active 1")?;
        self.connected = true;
        Ok(())
    }
    pub fn prepare(&mut self, round: u32, number: u32, field: u32) -> Result<()> {
        self.request(&format!("field 1 {field}"))?;
        let round = match round {
            1 => 8,
            2 => 1,
            6 => 2,
            3 => 3,
            4 => 4,
            5 => 5,
            _ => 99,
        };
        self.request(&format!("match {round} 1 {number}"))?;
        Ok(())
    }
    pub fn control(&mut self, mode: &str, seconds: u64) -> Result<()> {
        if !self.connected {
            bail!("Connect a V5 controller first")
        };
        if !["auto", "driver", "disable", "idle"].contains(&mode) {
            bail!("Invalid field-control mode")
        };
        self.request(&format!(
            "timer {}",
            seconds.saturating_mul(1000).min(i32::MAX as u64)
        ))?;
        self.request(mode)?;
        Ok(())
    }
    pub fn status(&mut self) -> Result<Value> {
        self.request("status").and_then(|v| {
            if v["watchdog_tripped"] == true {
                bail!("V5 watchdog disabled the controller after heartbeat loss")
            };
            if v["status"].is_null() {
                bail!("V5 controller status is unavailable or the controller disconnected")
            };
            Ok(v["status"].clone())
        })
    }
    pub fn disconnect(&mut self) {
        if self.connected {
            let _ = self.request("disable");
        }
        let _ = self.request("close");
        self.connected = false;
    }
}
impl Drop for V5 {
    fn drop(&mut self) {
        self.disconnect();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Read-only USB visibility, independent of a successful driver connection.
pub fn usb_diagnostics() -> Value {
    #[cfg(target_os="windows")]
    {return serde_json::json!({"available":false,"devices":[],"message":"Open Windows Device Manager to inspect USB and COM devices. The bundled V5 field-control helper handles controller discovery."});}

    let output = Command::new("/usr/sbin/system_profiler")
        .args(["SPUSBDataType", "-json"])
        .output();
    fn collect(v: &Value, out: &mut Vec<Value>) {
        if let Some(a) = v.as_array() {
            for x in a {
                collect(x, out)
            }
        }
        if let Some(o) = v.as_object() {
            if o.contains_key("vendor_id") || o.contains_key("product_id") {
                out.push(serde_json::json!({"name":v["_name"],"vendor":v["vendor_id"],"product":v["product_id"]}));
            }
            if let Some(items) = o.get("_items") {
                collect(items, out)
            }
            if let Some(items) = o.get("SPUSBDataType") {
                collect(items, out)
            }
        }
    }
    match output {
        Ok(o) if o.status.success() => match serde_json::from_slice::<Value>(&o.stdout) {
            Ok(v) => {
                let mut devices = vec![];
                collect(&v, &mut devices);
                serde_json::json!({"available":true,"devices":devices,"message":if devices.is_empty(){"macOS is not enumerating any external USB devices. Check Brain power, a data-capable USB cable, direct connection, and macOS accessory approval."}else{"macOS sees USB devices. Check whether a VEX Brain appears in the list. If it is absent, check its USB cable, power, and port. If present but driver count is zero, check field-control software, firmware, or another app holding the device."}})
            }
            Err(e) => {
                serde_json::json!({"available":false,"message":format!("Cannot read USB inventory: {e}")})
            }
        },
        _ => {
            serde_json::json!({"available":false,"message":"USB inventory could not be read; inspect System Information → USB."})
        }
    }
}
impl V5 {
    pub fn rescan(&mut self) -> Result<usize> {
        let reply = self.request("count")?;
        self.count = reply["count"].as_u64().unwrap_or(0) as usize;
        Ok(self.count)
    }
}

/// Prefer the driver shipped with this app, retaining installed-TM fallback for development.
pub fn default_driver() -> std::path::PathBuf {
    #[cfg(target_os="windows")]
    {return crate::platform::resources().unwrap_or_default().join("v5field-windows.so");}

    if let Ok(exe)=std::env::current_exe(){if let Some(contents)=exe.parent().and_then(|p|p.parent()){let path=contents.join("Resources/v5field-darwin-amd64.so");if path.is_file(){return path;}}}
    std::path::PathBuf::from("/Applications/VEX Tournament Manager.app/Contents/MacOS/v5field-darwin-amd64.so")
}
