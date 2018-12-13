use std::env;
use std::fs::File;
use std::process::Command;
use std::io::copy;
use dirs;
use env_perm;
use reqwest;

fn main() {
    match env::var("VULKAN_SDK") {
        // Vulkan SDK is installed, do nothing
        Ok(_) => return,
        // Install Vulkan SDK
        Err(_) =>(),
    }
    let mut dl_dir = dirs::download_dir().expect("No download dir");
    let address = "https://sdk.lunarg.com/sdk/download/latest/mac/vulkan-sdk.tar.gz";
    //let address = "http://0.0.0.0:8000/vulkan-sdk.tar.gz";
    let mut response = reqwest::get(address).expect("Failed to download sdk");

    let mut file_name: String;
    {
        let mut dest = {
            let fname = response
                .url()
                .path_segments()
                .and_then(|segments| segments.last())
                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                .unwrap_or("vulkansdk.tar.gz");

            eprintln!("file to download: '{}'", fname);
            file_name = fname.to_string();
            dl_dir.push(fname);
            eprintln!("will be located under: '{:?}'", dl_dir);
            File::create(&dl_dir).expect("failed to create file")
        };
        copy(&mut response, &mut dest).expect("Failed to write to file");
    }


    let home = dirs::home_dir().expect("No home directory found");
    let mut sdk_dir = home.clone();
    sdk_dir.push(".vulkan_sdk");
    
    let r = Command::new("mkdir")
        .arg(&sdk_dir)
        .output()
        .expect("failed to execute process");
    eprintln!("command: {:?}", r);

    let r = Command::new("mv")
        .arg(&dl_dir)
        .arg(&sdk_dir)
        .output()
        .expect("failed to execute process");
    eprintln!("command: {:?}", r);

    let r = Command::new("tar")
        .arg("-xzf")
        .arg(format!("{}/{}", sdk_dir.display(), file_name))
        .arg("-C")
        .arg(&sdk_dir)
        .arg("--strip-components=1")
        .output()
        .expect("failed to execute process");
    eprintln!("command: {:?}", r);
    let r = Command::new("rm")
        .arg(format!("{}/{}", sdk_dir.display(), file_name))
        .output()
        .expect("failed to execute process");
    eprintln!("command: {:?}", r);



    //export VULKAN_SDK=$HOME/vulkan_sdk/macOS
    env_perm::check_or_set("VULKAN_SDK", r#""$HOME/.vulkan_sdk/macOS""#).expect("Failed to set VULKAN_SDK");
    //export PATH=$VULKAN_SDK/bin:$PATH
    env_perm::append("PATH", "$VULKAN_SDK/bin").expect("Failed to append PATH");
    //export DYLD_LIBRARY_PATH=$VULKAN_SDK/lib:$DYLD_LIBRARY_PATH
    env_perm::append("DYLD_LIBRARY_PATH", "$VULKAN_SDK/lib").expect("Failed to append DYLD");
    //export VK_ICD_FILENAMES=$VULKAN_SDK/etc/vulkan/icd.d/MoltenVK_icd.json
    env_perm::check_or_set("VK_ICD_FILENAMES", r#""$VULKAN_SDK/etc/vulkan/icd.d/MoltenVK_icd.json""#)
        .expect("Failed to set VK_ICD_FILENAMES");
    //export VK_LAYER_PATH=$VULKAN_SDK/etc/vulkan/explicit_layer.d
    env_perm::check_or_set("VK_LAYER_PATH", r#""$VULKAN_SDK/etc/vulkan/explicit_layer.d""#)
        .expect("Failed to set VK_LAYER_PATH");
}
