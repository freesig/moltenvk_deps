use curl;
use curl::easy::Easy;
use dirs;
use env_perm;
use lazy_static;
use pbr::{ProgressBar, Units};
use std::env;
use std::fs::File;
use std::io::{self, copy};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tempdir::TempDir;
use std::io::Write;

const ADDRESS: &'static str = "https://sdk.lunarg.com/sdk/download/latest/mac/vulkan-sdk.tar.gz";

// The file size fallback
const FILE_SIZE: u64 = 209_715_200;

struct ProgressInfo {
    pf: Box<dyn FnMut(u64, u64) -> bool + Send>,
    file_size: u64,
}

lazy_static::lazy_static! {
    static ref PROGRESS_FUNCTION: Mutex<ProgressInfo> = {
        Mutex::new(
            ProgressInfo {
                pf: Box::new(|_, _| true),
                file_size: FILE_SIZE,
            })
    };
}

struct SDK {
    name: String,
    path: PathBuf,
    // this needs to not be dropped
    // yet or the directory is removed
    _tmp_dir: TempDir,
}

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    FailedCurlSetup(String),
    FailedSdkDownload,
    FailedCommand(String),
    FailedSetEnvVar,
    /// User has sdk in non default directory
    /// Recommend continuing silently
    NonDefaultDir,
    /// Env vars needed to be reset
    /// Probably the user has not sourced .bash_profile yet
    /// Recommend continuing silently
    ResetEnvVars(PathBuf),
    /// User has chosen not to install the sdk
    ChoseNotToInstall,
}

/// Either install silently
/// or with a call messages.
/// Can use Default::default()
/// which gives a default message
pub enum Install {
    Silent,
    Message(Message),
}

/// Specify callbacks to the user
/// for the install.
/// Can use Default::default()
pub struct Message {
    /// Initial question, do they want to install?
    pub question: Box<dyn FnMut() -> bool>,
    /// This function gives progress while the download is happending
    /// (download_so_for, total_file_size)
    pub progress: Box<dyn FnMut(u64, u64) -> bool + Send>,
    /// Message for when unpacking tar
    pub unpacking: Box<dyn FnMut()>,
    /// Message for when complete
    pub complete: Box<dyn FnMut()>,
}

impl Default for Install {
    fn default() -> Self {
        Install::Message(Default::default())
    }
}

fn progress_function(_: f64, downloaded: f64, _: f64, _: f64) -> bool {
    let mut p = PROGRESS_FUNCTION.lock().unwrap();
    let size: u64 = p.file_size as u64;
    (p.pf.as_mut())(downloaded as u64, size)
}

impl SDK {
    fn download() -> Result<Self, Error> {
        let tmp_dir = TempDir::new("sdk_download").map_err(|e| Error::IO(e))?;
        let mut handle = Easy::new();
        let mut response = Vec::new();
        handle
            .timeout(Duration::from_secs(0))
            .map_err(|_| Error::FailedCurlSetup("Set timeout failed".to_string()))?;
        handle
            .progress(true)
            .map_err(|_| Error::FailedCurlSetup("Set progress failed".to_string()))?;
        handle
            .progress_function(progress_function)
            .map_err(|_| Error::FailedCurlSetup("Set progress fucntion failed".to_string()))?;
        handle.url(ADDRESS).map_err(|_| Error::FailedSdkDownload)?;
        {
            let mut transfer = handle.transfer();
            transfer
                .write_function(|data| {
                    let len = data.len();
                    response.extend_from_slice(data);
                    Ok(len)
                })
                .map_err(|_| {
                    Error::FailedCurlSetup("Failed to create write function".to_string())
                })?;
            transfer.perform().map_err(|_| Error::FailedSdkDownload)?;
        }
        let (file, downloaded) = {
            let file_name = ADDRESS.split('/').last().unwrap_or("vulkan-sdk.tar.gz");

            let path = tmp_dir.path().join(&file_name);
            (
                File::create(&path),
                SDK {
                    name: file_name.into(),
                    path,
                    _tmp_dir: tmp_dir,
                },
            )
        };
        file.and_then(|mut dest| copy(&mut &response[..], &mut dest))
            .and_then(move |_| Ok(downloaded))
            .map_err(|e| Error::IO(e))
    }

    fn unpack(self) -> Result<(), Error> {
        let Self {
            name,
            path: dl_path,
            ..
        } = self;
        let home = dirs::home_dir().ok_or(Error::IO(io::ErrorKind::NotFound.into()))?;
        let mut sdk_dir = home.clone();
        sdk_dir.push(".vulkan_sdk");

        // Make the vulkan sdk directory
        Command::new("mkdir")
            .arg(&sdk_dir)
            .output()
            .map_err(|_| Error::FailedCommand("Failed to mkdir".to_string()))?;

        // Move the downloaded SDK there
        Command::new("mv")
            .arg(&dl_path)
            .arg(&sdk_dir)
            .output()
            .map_err(|_| Error::FailedCommand("Failed to mv".to_string()))?;

        // Untar the contents
        Command::new("tar")
            .arg("-xzf")
            .arg(format!("{}/{}", sdk_dir.display(), name))
            .arg("-C")
            .arg(&sdk_dir)
            .arg("--strip-components=1")
            .output()
            .map_err(|_| Error::FailedCommand("Failed to tar".to_string()))?;

        // Remove the empty dirctory
        Command::new("rm")
            .arg(format!("{}/{}", sdk_dir.display(), name))
            .output()
            .map_err(|_| Error::FailedCommand("Failed to rm".to_string()))?;

        println!(
            "The Vulkan SDK was successfully installed at {}",
            sdk_dir.display()
        );
        Ok(())
    }
}

fn set_env_vars() -> Result<(), Error> {
    println!("Setting environment variables");
    //export VULKAN_SDK=$HOME/vulkan_sdk/macOS
    env_perm::check_or_set("VULKAN_SDK", r#""$HOME/.vulkan_sdk/macOS""#)
        .map_err(|e| Error::IO(e))?;
    //export PATH=$VULKAN_SDK/bin:$PATH
    env_perm::append("PATH", "$VULKAN_SDK/bin").map_err(|e| Error::IO(e))?;
    //export DYLD_LIBRARY_PATH=$VULKAN_SDK/lib:$DYLD_LIBRARY_PATH
    env_perm::append("DYLD_LIBRARY_PATH", "$VULKAN_SDK/lib").map_err(|e| Error::IO(e))?;
    //export VK_ICD_FILENAMES=$VULKAN_SDK/etc/vulkan/icd.d/MoltenVK_icd.json
    env_perm::check_or_set(
        "VK_ICD_FILENAMES",
        r#""$VULKAN_SDK/etc/vulkan/icd.d/MoltenVK_icd.json""#,
    )
    .map_err(|e| Error::IO(e))?;
    //export VK_LAYER_PATH=$VULKAN_SDK/etc/vulkan/explicit_layer.d
    env_perm::check_or_set(
        "VK_LAYER_PATH",
        r#""$VULKAN_SDK/etc/vulkan/explicit_layer.d""#,
    )
    .map_err(|e| Error::IO(e))?;
    //export SHADERC_LIB_DIR=$VULKAN_SDK/lib
    env_perm::check_or_set("SHADERC_LIB_DIR", r#""$VULKAN_SDK/lib""#).map_err(|e| Error::IO(e))?;
    set_temp_envs()?;
    Ok(())
}

// Sets the environment variables temporarily because the
// use has not source'd the .bash_profile yet.
fn set_temp_envs() -> Result<(), Error> {
    //export VULKAN_SDK=$HOME/vulkan_sdk/macOS
    let mut vulkan_sdk = dirs::home_dir().ok_or(Error::IO(io::ErrorKind::NotFound.into()))?;
    vulkan_sdk.push(".vulkan_sdk");
    vulkan_sdk.push("macOS");
    env::set_var("VULKAN_SDK", vulkan_sdk.clone().into_os_string());

    //export PATH=$VULKAN_SDK/bin:$PATH
    let mut bin = vulkan_sdk.clone();
    bin.push("bin");
    if let Some(mut paths) = get_current_path() {
        paths.push(bin);
        let new_path = env::join_paths(paths).map_err(|_| Error::FailedSetEnvVar)?;
        env::set_var("PATH", &new_path);
    }

    //export DYLD_LIBRARY_PATH=$VULKAN_SDK/lib:$DYLD_LIBRARY_PATH
    let mut lib = vulkan_sdk.clone();
    lib.push("lib");
    if let Some(dyld) = env::var_os("DYLD_LIBRARY_PATH") {
        let mut libs = env::split_paths(&dyld).collect::<Vec<_>>();
        libs.push(lib);
        let new_dyld = env::join_paths(libs).map_err(|_| Error::FailedSetEnvVar)?;
        env::set_var("DYLD_LIBRARY_PATH", &new_dyld);
    }

    let mut icd = vulkan_sdk.clone();
    icd.push("etc");
    icd.push("vulkan");
    icd.push("icd.d");
    icd.push("MoltenVK_icd.json");
    //export VK_ICD_FILENAMES=$VULKAN_SDK/etc/vulkan/icd.d/MoltenVK_icd.json
    env::set_var("VK_ICD_FILENAMES", icd.into_os_string());

    //export VK_LAYER_PATH=$VULKAN_SDK/etc/vulkan/explicit_layer.d
    let mut layer = vulkan_sdk.clone();
    layer.push("etc");
    layer.push("vulkan");
    layer.push("explicit_layer.d");
    env::set_var("VK_LAYER_PATH", layer.into_os_string());

    //export SHADERC_LIB_DIR=$VULKAN_SDK/lib
    let mut clib = vulkan_sdk.clone();
    clib.push("lib");
    env::set_var("SHADERC_LIB_DIR", clib.into_os_string());
    Ok(())
}

fn get_current_path() -> Option<Vec<PathBuf>> {
    Command::new("bash")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()
        .and_then(|mut output| {
            output
                .stdin
                .as_mut()
                .and_then(|stdin| {
                    stdin
                        .write_all(b"source ~/.bash_profile\n")
                        .ok()
                        .and_then(|_| stdin.write_all(b"echo $PATH").ok())
                })
                .and_then(|_| {
                    output.wait_with_output().ok().map(|output| {
                        let path = String::from_utf8_lossy(&output.stdout);
                        env::split_paths(&path.trim()).collect::<Vec<_>>()
                    })
                })
        })
}

// Is the default sdk directory empty
fn check_sdk_dir() -> Result<bool, Error> {
    let mut sdk_dir = dirs::home_dir().ok_or(Error::IO(io::ErrorKind::NotFound.into()))?;
    sdk_dir.push(".vulkan_sdk");
    Ok(sdk_dir.exists())
}

// Is the VULKAN_SDK variable pointing at
// the default location and is that location empty.
fn is_default_dir_and_empty(vulkan_sdk: String) -> Result<bool, Error> {
    let mut default_dir = dirs::home_dir().ok_or(Error::IO(io::ErrorKind::NotFound.into()))?;
    default_dir.push(".vulkan_sdk");
    default_dir.push("macOS");
    Ok(vulkan_sdk == default_dir.to_string_lossy() && !default_dir.exists())
}

fn default_lib_dir() -> Result<PathBuf, Error> {
    let mut lib_dir = dirs::home_dir().ok_or(Error::IO(io::ErrorKind::NotFound.into()))?;
    lib_dir.push(".vulkan_sdk");
    lib_dir.push("macOS");
    lib_dir.push("lib");
    lib_dir.push("libvulkan.1.dylib");
    Ok(lib_dir)
}

impl Default for Message {
    fn default() -> Self {
        let question = Box::new(|| {
            println!("Vulkano requires the Vulkan SDK to use MoltenVK for MacOS");
            println!(
                "The SDK will now be downloaded and environment variables added to .bash_profile"
            );
            true
        });
        let mut bar = ProgressBar::new(FILE_SIZE as u64);
        bar.set_units(Units::Bytes);
        let progress = Box::new(move |downloaded, _| {
            bar.set(downloaded);
            true
        });
        let unpacking = Box::new(|| {
            println!("Unpacking file into ~/.vulkan_sdk");
        });
        let complete = Box::new(|| {
            println!("Installation complete :D");
            println!("To update simply remove the '~/.vulkan_sdk' directory");
        });
        Message {
            question,
            progress,
            unpacking,
            complete,
        }
    }
}

/// This will check if you have the
/// Vulkan SDK installed by checking
/// if the VULKAN_SDK env var is set.
/// If it's set then nothing will happen.
/// If it is not set then it will download
/// the latest SDK from lunarg.com and install
/// it at home/.vulkan_sdk.
/// It will then set the required environmnet
/// variables.
/// You can install silently or with a message.
/// Use `check_or_install(Default::default())`
/// for an install with a default message
pub fn check_or_install(install: Install) -> Result<PathBuf, Error> {
    match env::var("VULKAN_SDK") {
        // VULKAN_SDK is set
        Ok(v) => {
            if is_default_dir_and_empty(v)? {
                // Install as the directory is empty
            } else {
                // VULKAN_SDK is set to a non-default or directory is not empty.
                // Return silently.
                return Err(Error::NonDefaultDir);
            }
        }
        Err(_) => {
            // Environment Variables are not set
            // but might just need to be set temporarily.
            if check_sdk_dir()? {
                // Set env vars and return silently.
                set_temp_envs()?;
                return Err(Error::ResetEnvVars(default_lib_dir()?));
            }
            // Vulkan SDK needs to be installed
        }
    }

    match install {
        Install::Silent => {
            let sdk = SDK::download()?;
            sdk.unpack()?;
            set_env_vars()?;
        }
        Install::Message(mut message) => {
            {
                PROGRESS_FUNCTION.lock().unwrap().pf = message.progress;
            }
            if (message.question)() {
                let sdk = SDK::download()?;
                (message.unpacking)();
                sdk.unpack()?;
                set_env_vars()?;

                (message.complete)();
            } else {
                return Err(Error::ChoseNotToInstall);
            }
        }
    }

    Ok(default_lib_dir()?)
}
