use std::env;
use std::fs::File;
use std::process::Command;
use std::io::{self, copy};
use std::path::PathBuf;
use dirs;
use env_perm;
use reqwest;
use tempdir::TempDir;

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
        let mut response = reqwest::get(ADDRESS).expect("Failed to download sdk");
        let (file, downloaded) = {
            let file_name = response
                .url()
                .path_segments()
                .and_then(|segments| segments.last())
                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                .unwrap_or("vulkansdk.tar.gz");

            let path = tmp_dir.path().join(&file_name);
            (File::create(&path),
             SDK{ name: file_name.into(), path, _tmp_dir: tmp_dir })
        };
        file.and_then(|mut dest| copy(&mut response, &mut dest))
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
        let r = Command::new("mkdir")
            .arg(&sdk_dir)
            .output()
            .expect("failed to execute process");
        eprintln!("command: {:?}", r);

        // Move the downloaded SDK there
        let r = Command::new("mv")
            .arg(&dl_path)
            .arg(&sdk_dir)
            .output()
            .expect("failed to execute process");
        eprintln!("command: {:?}", r);

        // Untar the contents
        let r = Command::new("tar")
            .arg("-xzf")
            .arg(format!("{}/{}", sdk_dir.display(), name))
            .arg("-C")
            .arg(&sdk_dir)
            .arg("--strip-components=1")
            .output()
            .expect("failed to execute process");
        eprintln!("command: {:?}", r);

        // Remove the empty dirctory
        let r = Command::new("rm")
            .arg(format!("{}/{}", sdk_dir.display(), name))
            .output()
            .expect("failed to execute process");
        eprintln!("command: {:?}", r);
        Ok(())
    }
}

fn set_env_vars() -> io::Result<()> {
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
    Ok(())
}

pub fn check_or_install() {
    match env::var("VULKAN_SDK") {
        // Vulkan SDK is installed, do nothing
        Ok(_) => return,
        // Install Vulkan SDK
        Err(_) =>(),
    }

    let sdk = SDK::download().expect("Downloading the Vulkan SDK failed");
    sdk.unpack().expect("Failed to unpack the Vulkan SDK");

    set_env_vars().expect("Failed to set the required environment variables");
}
