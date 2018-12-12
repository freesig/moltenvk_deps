use std::env;
use std::fs::File;
use std::process::Command;
use std::io::Write;
use dirs;
use curl::easy::Easy;
use env_perm;

fn main() {
    match env::var("VULKAN_SDK2") {
        // Vulkan SDK is installed, do nothing
        Ok(_) => return,
        // Install Vulkan SDK
        Err(_) =>(),
    }
    {
        let mut dl_dir = dirs::download_dir().expect("No download dir");
        dl_dir.push("vulkansdk-macos-1.1.92.1.tar.gz");
        let mut sdk = File::create(dl_dir).expect("Failed to create sdk tar");
        let mut easy = Easy::new();
        // TODO verify hash
        easy.url("https://vulkan.lunarg.com/sdk/home#sdk/downloadConfirm/1.1.92.1/mac/vulkansdk-macos-1.1.92.1.tar.gz")
            .expect("URL failed");
        easy.write_function(move |data| {
            sdk.write_all(data).expect("Failed to write");
            Ok(data.len())
        }).unwrap();
        easy.perform().expect("Failed to perform curl");

        println!("{}", easy.response_code().unwrap());
    }

    Command::new("bash")
            .arg("mkdir")
            .arg("~/.vulkan_sdk")
            .output()
            .expect("failed to execute process");
    
    Command::new("bash")
            .arg("mv")
            .arg("~/Downloads/vulkansdk-macos-1.1.92.1.tar.gz ~/.vulkan_sdk")
            .output()
            .expect("failed to execute process");
    
    Command::new("bash")
            .arg("cd")
            .arg("~/.vulkan_sdk")
            .arg("&&")
            .arg("tar")
            .arg("-xzf")
            .arg("vulkansdk-macos-1.1.92.1.tar.gz")
            .output()
            .expect("failed to execute process");

    Command::new("bash")
            .arg("cd")
            .arg("~/.vulkan_sdk")
            .arg("&&")
            .arg("mv")
            .arg("-v")
            .arg("vulkansdk-macos-1.1.92.1/* .")
            .output()
            .expect("failed to execute process");
    
    //export VULKAN_SDK=$HOME/vulkan_sdk/macOS
    env_perm::check_or_set("VULKAN_SDK", r#""$HOME/vulkan_sdk/macOS""#).expect("Failed to set VULKAN_SDK");
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
