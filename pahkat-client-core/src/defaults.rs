use crate::config::ConfigPath;
use directories::BaseDirs;
use std::path::{Path, PathBuf};
use url::Url;

#[cfg(not(target_os = "android"))]
pub fn config_path() -> Option<PathBuf> {
    if cfg!(windows) && whoami::username() == "SYSTEM" {
        // TODO: Do not hardcode this.
        return Some(Path::new(r"C:\ProgramData\Pahkat\config").to_path_buf());
    }
    
    if cfg!(target_os = "macos") && whoami::username() == "root" {
        Some(Path::new(r"/Library/Preferences/Pahkat").to_path_buf())
    } else {
        BaseDirs::new().map(|x| x.config_dir().join("Pahkat").to_path_buf())
    }
}

#[inline(always)]
#[cfg(not(target_os = "android"))]
fn raw_cache_dir() -> Option<PathBuf> {
    if cfg!(windows) && whoami::username() == "SYSTEM" {
        // TODO: Do not hardcode this.
        return Some(Path::new(r"C:\ProgramData\Pahkat\cache").to_path_buf());
    }
    
    if cfg!(target_os = "macos") && whoami::username() == "root" {
        Some(Path::new(r"/Library/Caches/Pahkat").to_path_buf())
    } else {
        let dir = BaseDirs::new().map(|x| x.cache_dir().components().as_path().to_path_buf())?;
        std::fs::create_dir_all(&dir).ok()?;
        dir.canonicalize().ok()
    }
}

#[inline(always)]
pub fn log_path() -> Option<PathBuf> {
    if cfg!(windows) && whoami::username() == "SYSTEM" {
        // TODO: Do not hardcode this.
        return Some(Path::new(r"C:\ProgramData\Pahkat\log").to_path_buf())
    }
    
    if cfg!(target_os = "macos") && whoami::username() == "root" {
        Some(Path::new(r"/Library/Logs/Pahkat").to_path_buf())
    } else {
        None
    }
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub fn cache_dir() -> ConfigPath {
    ConfigPath::File(Url::from_directory_path(&raw_cache_dir().unwrap()).unwrap())
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub fn tmp_dir() -> ConfigPath {
    let url = Url::from_file_path(&raw_cache_dir().unwrap().join("tmp")).unwrap();
    ConfigPath::File(url)
}

#[cfg(target_os = "ios")]
pub fn cache_dir() -> ConfigPath {
    let url = Url::parse("container:/Library/Caches/Pahkat").unwrap();
    ConfigPath::Container(url)
}

#[cfg(target_os = "ios")]
pub fn tmp_dir() -> ConfigPath {
    let url = Url::parse("container:/Library/Caches/Pahkat/tmp").unwrap();
    ConfigPath::Container(url)
}

#[cfg(target_os = "android")]
pub fn cache_dir() -> ConfigPath {
    let url = Url::parse("container:/cache/Pahkat").unwrap();
    ConfigPath::Container(url)
}

#[cfg(target_os = "android")]
pub fn tmp_dir() -> ConfigPath {
    let url = Url::from_directory_path(std::env::temp_dir()).unwrap();
    ConfigPath::File(url)
}

#[cfg(target_os = "macos")]
pub fn uninstall_path() -> PathBuf {
    BaseDirs::new()
        .expect("base directories must be known")
        .data_dir()
        .join("Pahkat")
        .join("uninstall")
}

macro_rules! platform {
    ($name:expr) => {{
        #[cfg(target_os = $name)]
        {
            return $name;
        }
    }};
}

#[inline(always)]
#[allow(unreachable_code)]
pub(crate) const fn platform() -> &'static str {
    platform!("windows");
    platform!("macos");
    platform!("ios");
    platform!("android");
    platform!("linux");
}

macro_rules! arch {
    ($name:expr) => {{
        #[cfg(target_arch = $name)]
        {
            return Some($name);
        }
    }};
}

#[inline(always)]
#[allow(unreachable_code)]
pub(crate) const fn arch() -> Option<&'static str> {
    arch!("x86_64");
    arch!("x86");
    arch!("arm");
    arch!("aarch64");
    arch!("mips");
    arch!("mips64");
    arch!("powerpc");
    arch!("powerpc64");
}

#[inline(always)]
pub(crate) fn payloads() -> &'static [&'static str] {
    #[cfg(all(feature = "windows", not(feature = "macos"), not(feature = "prefix")))]
    {
        &["WindowsExecutable"]
    }
    #[cfg(all(not(feature = "windows"), feature = "macos", not(feature = "prefix")))]
    {
        &["MacOSPackage"]
    }
    #[cfg(all(not(feature = "windows"), not(feature = "macos"), feature = "prefix"))]
    {
        &["TarballPackage"]
    }

    #[cfg(all(
        not(feature = "windows"),
        not(feature = "macos"),
        not(feature = "prefix")
    ))]
    compile_error!("One of the above features must be enabled");
}
