#[cfg(windows)]
mod platform {
    use std::io;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
    use winreg::RegKey;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "ModelD2ProBattery";

    pub fn is_enabled() -> io::Result<bool> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ)?;
        let expected = current_exe_value()?;
        let actual: Result<String, _> = key.get_value(VALUE_NAME);
        Ok(actual.map(|value| value == expected).unwrap_or(false))
    }

    pub fn set_enabled(enabled: bool) -> io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)?;
        if enabled {
            key.set_value(VALUE_NAME, &current_exe_value()?)?;
        } else {
            match key.delete_value(VALUE_NAME) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    fn current_exe_value() -> io::Result<String> {
        Ok(format!("\"{}\"", std::env::current_exe()?.display()))
    }
}

#[cfg(not(windows))]
mod platform {
    use std::io;

    pub fn is_enabled() -> io::Result<bool> {
        Ok(false)
    }

    pub fn set_enabled(_enabled: bool) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "startup toggle is only supported on Windows",
        ))
    }
}

pub use platform::*;
