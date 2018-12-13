# moltenvk_deps
Gets Macos dependencies for [MoltenVK](https://github.com/KhronosGroup/MoltenVK) and [Volkano-rs](https://github.com/vulkano-rs/vulkano).
Apple do not directly support Vulkan so we need [MoltenVK](https://github.com/KhronosGroup/MoltenVK)
in order to bind to Metal.
This allows Macos users to automatically get the requirements to use [Volkano-rs](https://github.com/vulkano-rs/vulkano).

This crate will check you have the Vulkan SDK from [Lunar](https://vulkan.lunarg.com/sdk/home) installed 
and the required environment variables set.

If you don't have them it will download and upack sdk.
Then it will set the environment variables permanently in your `.profile` or `.bash_profile`

__It will set:__
VULKAN_SDK=$HOME/vulkan_sdk/macOS
PATH=$VULKAN_SDK/bin:$PATH
DYLD_LIBRARY_PATH=$VULKAN_SDK/lib:$DYLD_LIBRARY_PATH
VK_ICD_FILENAMES=$VULKAN_SDK/etc/vulkan/icd.d/MoltenVK_icd.json
VK_LAYER_PATH=$VULKAN_SDK/etc/vulkan/explicit_layer.d
