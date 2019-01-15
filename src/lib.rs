use std::env;
use std::fs::File;
use std::process::Command;
use std::io::{self, copy};
use std::path::PathBuf;
use dirs;
use env_perm;
use curl;
use curl::easy::Easy; 
use tempdir::TempDir;
use std::time::Duration;

const ADDRESS: &'static str = "https://sdk.lunarg.com/sdk/download/latest/mac/vulkan-sdk.tar.gz";

struct SDK {
    name: String,
    path: PathBuf,
    // this needs to not be dropped
    // yet or the directory is removed
    _tmp_dir: TempDir,
}

impl SDK {
    fn download() -> io::Result<Self> {
        let tmp_dir = TempDir::new("sdk_download").expect("Failed to create temp directory");
        let mut handle = Easy::new();
        handle.progress(true).expect("Failed to set progress bar");
        let mut response = Vec::new();
        handle.timeout(Duration::from_secs(0)).expect("Set timeout failed");
        handle.url(ADDRESS).expect("Failed to download sdk");
        {
            let mut transfer = handle.transfer();
            transfer.write_function(|data| {
                let len = data.len();
                response.extend_from_slice(data);
                Ok(len)
            }).expect("Failed to create write function");
            transfer.perform().expect("Failed to perform transfer");
        }
        let (file, downloaded) = {
            let file_name = ADDRESS.split('/')
                .last()
                .unwrap_or("vulkan-sdk.tar.gz");

            let path = tmp_dir.path().join(&file_name);
            (File::create(&path),
             SDK{ name: file_name.into(), path, _tmp_dir: tmp_dir })
        };
        file.and_then(|mut dest| copy(&mut &response[..], &mut dest))
            .and_then(move |_| Ok(downloaded) )
    }

    fn unpack(self) -> io::Result<()> {
        let Self {
            name,
            path: dl_path,
            ..
        } = self;
        let home = dirs::home_dir().ok_or(io::ErrorKind::NotFound)?;
        let mut sdk_dir = home.clone();
        sdk_dir.push(".vulkan_sdk");

        // Make the vulkan sdk directory
        Command::new("mkdir")
            .arg(&sdk_dir)
            .output()
            .expect("failed to execute process");

        // Move the downloaded SDK there
        Command::new("mv")
            .arg(&dl_path)
            .arg(&sdk_dir)
            .output()
            .expect("failed to execute process");

        // Untar the contents
        Command::new("tar")
            .arg("-xzf")
            .arg(format!("{}/{}", sdk_dir.display(), name))
            .arg("-C")
            .arg(&sdk_dir)
            .arg("--strip-components=1")
            .output()
            .expect("failed to execute process");

        // Remove the empty dirctory
        Command::new("rm")
            .arg(format!("{}/{}", sdk_dir.display(), name))
            .output()
            .expect("failed to execute process");
        
        println!("The Vulkan SDK was successfully installed at {}", sdk_dir.display());
        Ok(())
    }
}

fn set_env_vars() -> io::Result<()> {
    println!("Setting environment variables");
    //export VULKAN_SDK=$HOME/vulkan_sdk/macOS
    env_perm::check_or_set("VULKAN_SDK", r#""$HOME/.vulkan_sdk/macOS""#)?;
    //export PATH=$VULKAN_SDK/bin:$PATH
    env_perm::append("PATH", "$VULKAN_SDK/bin")?;
    //export DYLD_LIBRARY_PATH=$VULKAN_SDK/lib:$DYLD_LIBRARY_PATH
    env_perm::append("DYLD_LIBRARY_PATH", "$VULKAN_SDK/lib")?;
    //export VK_ICD_FILENAMES=$VULKAN_SDK/etc/vulkan/icd.d/MoltenVK_icd.json
    env_perm::check_or_set("VK_ICD_FILENAMES", r#""$VULKAN_SDK/etc/vulkan/icd.d/MoltenVK_icd.json""#)?;
    //export VK_LAYER_PATH=$VULKAN_SDK/etc/vulkan/explicit_layer.d
    env_perm::check_or_set("VK_LAYER_PATH", r#""$VULKAN_SDK/etc/vulkan/explicit_layer.d""#)?;
    set_temp_envs();
    Ok(())
}

fn set_temp_envs() {
    //export VULKAN_SDK=$HOME/vulkan_sdk/macOS
    let mut vulkan_sdk = dirs::home_dir().expect("Failed to find home directory");
    vulkan_sdk.push(".vulkan_sdk");
    vulkan_sdk.push("macOS");
    env::set_var("VULKAN_SDK", vulkan_sdk.clone().into_os_string());

    //export PATH=$VULKAN_SDK/bin:$PATH
    let mut bin = vulkan_sdk.clone();
    bin.push("bin");
    if let Some(path) = env::var_os("PATH") {
        let mut paths = env::split_paths(&path).collect::<Vec<_>>();
        paths.push(bin);
        let new_path = env::join_paths(paths).expect("Failed to append to PATH");
        env::set_var("PATH", &new_path);
    }
    
    //export DYLD_LIBRARY_PATH=$VULKAN_SDK/lib:$DYLD_LIBRARY_PATH
    let mut lib = vulkan_sdk.clone();
    lib.push("lib");
    let lib_path = lib.clone();
    if let Some(dyld) = env::var_os("DYLD_LIBRARY_PATH") {
        let mut libs = env::split_paths(&dyld).collect::<Vec<_>>();
        libs.push(lib);
        let new_dyld = env::join_paths(libs).expect("Failed to append to DYLD_LIBRARY_PATH");
        env::set_var("DYLD_LIBRARY_PATH", &new_dyld);
    }

    // Temporary tell vulkano where the lib is
    env::set_var("VULKAN_LIB_PATH", lib_path.into_os_string());
    
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
}

fn check_sdk_dir() -> bool {
    let mut sdk_dir = dirs::home_dir().expect("Failed to find home directory");
    sdk_dir.push(".vulkan_sdk");
    sdk_dir.exists()
}

fn is_default_dir(vulkan_sdk: String) -> bool {
    let mut default_dir = dirs::home_dir().expect("Failed to find home directory");
    default_dir.push(".vulkan_sdk");
    default_dir.push("macOS");
    vulkan_sdk == default_dir.to_string_lossy()
}

/*

fn remove_old_sdk() -> io::Result<()> {
    let mut vulkan_sdk = dirs::home_dir().expect("Failed to find home directory");
    vulkan_sdk.push(".vulkan_sdk");
    if vulkan_sdk.exists() {
        // Move the downloaded SDK there
        Command::new("rm")
            .arg("-fr")
            .arg(&vulkan_sdk)
            .output()
            .expect("failed to execute process");
        Ok(())
    } else {
        println!("The SDK is not installed in the default location:");
        println!("{}", vulkan_sdk.display());
        println!("Automatic updates are only supported for the default location");
        Err(io::Error::new(io::ErrorKind::NotFound, "Directory missing"))
    }
}

fn update_sdk() {
    println!("Updating Vulkan SDK. This may take some time. Grab another coffer :)");
    match remove_old_sdk() {
        Ok(_) => {
            let sdk = SDK::download().expect("Downloading the Vulkan SDK failed");
            sdk.unpack().expect("Failed to unpack the Vulkan SDK");
            println!("Installation complete :D");
        },
        Err(_) => println!("SDK not updated"),
    }
}
*/

/// This will check if you have the
/// Vulkan SDK installed by checking
/// if the VULKAN_SDK env var is set.
/// If it's set then nothing will happen.
/// If it is not set then it will download
/// the latest SDK from lunarg.com and install
/// it at home/.vulkan_sdk.
/// It will then set the required environmnet 
/// variables.
pub fn check_or_install() {
    /*
    if env::var_os("UPDATE_VULKAN_SDK").is_some() {
        update_sdk();
    }
    */
    match env::var("VULKAN_SDK") {
        // Vulkan SDK is installed, do nothing
        Ok(v) => {
            if is_default_dir(v) && check_sdk_dir() {
                return;
            }
        },
        // Install Vulkan SDK
        Err(_) =>{
            if check_sdk_dir() {
                set_temp_envs();
                return;
            }
        },
    }

    println!("Vulkano requires the Vulkan SDK to use MoltenVK for MacOS");
    println!("Would you like to automatically install it now? (Y/n)");
    
    loop {
        let mut answer = String::new();
        io::stdin().read_line(&mut answer).expect("failed to read input");
        answer.pop();
        match answer.as_str() {
            "Y" => break,
            "n" => return,
            _ => println!("Invalid answer, enter 'Y' to install or 'n' to quit"),
        }
    }

    println!("Downloading and installing Vulkan SDK, This may take some time. Grab a coffee :)");

    let sdk = SDK::download().expect("Downloading the Vulkan SDK failed");
    sdk.unpack().expect("Failed to unpack the Vulkan SDK");

    set_env_vars().expect("Failed to set the required environment variables");
    println!("Installation complete :D");
    println!("To update run 'UPDATE_VULKAN_SDK=1 cargo run`");
}
