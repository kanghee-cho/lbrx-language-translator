#[cfg(windows)]
pub fn clipboard_revision() -> u64 {
    unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() as u64 }
}

#[cfg(target_os = "macos")]
pub fn clipboard_revision() -> u64 {
    objc2_app_kit::NSPasteboard::generalPasteboard().changeCount() as u64
}

#[cfg(not(any(windows, target_os = "macos")))]
compile_error!("LBRX Translator currently supports Windows and macOS only.");

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

pub fn input_permission() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        AXIsProcessTrusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

pub fn open_permissions() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("/usr/bin/open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .status()
            .map_err(|error| error.to_string())?;
        if !status.success() {
            return Err("macOS 손쉬운 사용 설정을 열지 못했습니다.".into());
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Windows에서는 별도의 입력 권한 설정이 필요하지 않습니다.".into())
    }
}
