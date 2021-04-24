#![allow(dead_code)]
#![allow(non_snake_case)]

use vulkan_bindings as vk;

use std::fmt;
use std::mem;
use std::ptr;

use std::ffi::CStr;
use std::ffi::CString;

pub fn string_from_c_str(c_str: &[i8]) -> String {
    let s = unsafe { CStr::from_ptr(c_str.as_ptr()).to_bytes() };
    String::from_utf8_lossy(s).into_owned()
}

#[cfg(target_os = "linux")]
const VULKAN_LIB: &str = "libvulkan.so.1";

#[cfg(windows)]
const VULKAN_LIB: &str = "vulkan-1.dll";

pub struct Vulkan {
    lib: libloading::Library,
    pub GetInstanceProcAddr: vk::FnGetInstanceProcAddr,
    pub commands: vk::LibraryCommands,
}

#[repr(transparent)]
pub struct VkExtensionProperties(pub vk::ExtensionProperties);

impl VkExtensionProperties {
    fn default() -> Self {
        VkExtensionProperties(vk::ExtensionProperties {
            extensionName: [0; vk::MAX_EXTENSION_NAME_SIZE as _],
            specVersion: 0,
        })
    }

    pub fn extension_name(&self) -> String {
        string_from_c_str(&self.0.extensionName)
    }
}

#[repr(transparent)]
pub struct VkLayerProperties(pub vk::LayerProperties);

impl VkLayerProperties {
    fn default() -> Self {
        VkLayerProperties(vk::LayerProperties {
            layerName: [0; vk::MAX_EXTENSION_NAME_SIZE as usize],
            specVersion: 0,
            implementationVersion: 0,
            description: [0; vk::MAX_DESCRIPTION_SIZE as usize],
        })
    }

    pub fn layer_name(&self) -> String {
        string_from_c_str(&self.0.layerName)
    }
    pub fn description(&self) -> String {
        string_from_c_str(&self.0.description)
    }
}

impl Clone for VkExtensionProperties {
    fn clone(&self) -> Self {
        Self(vk::ExtensionProperties {
            extensionName: self.0.extensionName,
            specVersion: self.0.specVersion,
        })
    }
}

impl Clone for VkLayerProperties {
    fn clone(&self) -> Self {
        Self(vk::LayerProperties {
            layerName: self.0.layerName,
            specVersion: self.0.specVersion,
            implementationVersion: self.0.implementationVersion,
            description: self.0.description,
        })
    }
}

impl Vulkan {
    pub fn new() -> Result<Self, libloading::Error> {
        let maybe_lib = unsafe { libloading::Library::new(VULKAN_LIB) };
        maybe_lib.map(|lib| {
            let GetInstanceProcAddr: vk::FnGetInstanceProcAddr = unsafe {
                let s = lib
                    .get::<vk::FnVoidFunction>(b"vkGetInstanceProcAddr\0")
                    .unwrap();
                mem::transmute(s.into_raw())
            };
            let commands = vk::LibraryCommands::new(GetInstanceProcAddr, vk::NULL_HANDLE);
            Vulkan {
                lib,
                GetInstanceProcAddr,
                commands,
            }
        })
    }

    pub fn enum_extensions(&self) -> Result<Vec<VkExtensionProperties>, vk::Result> {
        let mut num_properties: u32 = 0;
        let result = unsafe {
            self.commands.EnumerateInstanceExtensionProperties(
                ptr::null(),
                &mut num_properties,
                ptr::null_mut(),
            )
        };

        if result == vk::SUCCESS {
            let mut ext_props = vec![VkExtensionProperties::default(); num_properties as _];
            let result = unsafe {
                self.commands.EnumerateInstanceExtensionProperties(
                    ptr::null(),
                    &mut num_properties,
                    ext_props.as_mut_ptr() as _,
                )
            };

            if result == vk::SUCCESS {
                Ok(ext_props)
            } else {
                Err(result)
            }
        } else {
            Err(result)
        }
    }

    pub fn enum_layers(&self) -> Result<Vec<VkLayerProperties>, vk::Result> {
        let mut num_properties: u32 = 0;
        let result = unsafe {
            self.commands
                .EnumerateInstanceLayerProperties(&mut num_properties, ptr::null_mut())
        };

        if result == vk::SUCCESS {
            let mut layer_props = vec![VkLayerProperties::default(); num_properties as _];
            let result = unsafe {
                self.commands.EnumerateInstanceLayerProperties(
                    &mut num_properties,
                    layer_props.as_mut_ptr() as _,
                )
            };

            if result == vk::SUCCESS {
                Ok(layer_props)
            } else {
                Err(result)
            }
        } else {
            Err(result)
        }
    }
}

pub struct Instance {
    pub vk: Vulkan,
    pub instance: vk::Instance,
    pub commands: vk::InstanceCommands,
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { self.commands.DestroyInstance(self.instance, ptr::null()) };
    }
}

impl Instance {
    pub fn new(
        vk: Vulkan,
        app_name: &str,
        engine_name: &str,
        layers: &[&str],
        extensions: &[&str],
    ) -> Result<Instance, vk::Result> {
        let app_name_cstr = CString::new(app_name).unwrap();
        let engine_name_cstr = CString::new(engine_name).unwrap();

        let app_info = vk::ApplicationInfo {
            sType: vk::STRUCTURE_TYPE_APPLICATION_INFO,
            pNext: ptr::null(),
            pApplicationName: app_name_cstr.as_ptr(),
            applicationVersion: 1,
            pEngineName: engine_name_cstr.as_ptr(),
            engineVersion: 1,
            apiVersion: vk::make_version(1, 2, 133),
        };

        let layers_cstr: Vec<_> = layers.iter().map(|&s| CString::new(s).unwrap()).collect();
        let extensions_cstr: Vec<_> = extensions
            .iter()
            .map(|&s| CString::new(s).unwrap())
            .collect();

        let layers_ptr: Vec<_> = layers_cstr.iter().map(|s| s.as_ptr()).collect();
        let extensions_ptr: Vec<_> = extensions_cstr.iter().map(|s| s.as_ptr()).collect();

        let instance_info = vk::InstanceCreateInfo {
            sType: vk::STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
            pNext: ptr::null(),
            flags: 0,
            pApplicationInfo: &app_info,
            enabledLayerCount: layers.len() as _,
            ppEnabledLayerNames: layers_ptr.as_ptr(),
            enabledExtensionCount: extensions.len() as _,
            ppEnabledExtensionNames: extensions_ptr.as_ptr(),
        };

        let mut instance: vk::Instance = 0;
        let result = unsafe {
            vk.commands
                .CreateInstance(&instance_info, ptr::null(), &mut instance)
        };

        match result {
            vk::SUCCESS => {
                let commands = vk::InstanceCommands::new(vk.GetInstanceProcAddr, instance);
                Ok(Instance {
                    vk,
                    instance,
                    commands,
                })
            }
            _ => Err(result),
        }
    }

    fn enum_physical_devices(&self) -> Result<Vec<vk::PhysicalDevice>, vk::Result> {
        let mut num_devices: u32 = 0;
        let result = unsafe {
            self.commands
                .EnumeratePhysicalDevices(self.instance, &mut num_devices, ptr::null_mut())
        };

        if result == vk::SUCCESS {
            let mut devices = vec![vk::PhysicalDevice::default(); num_devices as _];
            let result = unsafe {
                self.commands.EnumeratePhysicalDevices(
                    self.instance,
                    &mut num_devices,
                    devices.as_mut_ptr() as _,
                )
            };

            if result == vk::SUCCESS {
                Ok(devices)
            } else {
                Err(result)
            }
        } else {
            Err(result)
        }
    }

    pub fn enum_physical_device_extensions(
        &self,
        device: vk::PhysicalDevice,
    ) -> Result<Vec<VkExtensionProperties>, vk::Result> {
        let mut num_properties: u32 = 0;
        let result = unsafe {
            self.commands.EnumerateDeviceExtensionProperties(
                device,
                ptr::null(),
                &mut num_properties,
                ptr::null_mut(),
            )
        };

        if result == vk::SUCCESS {
            let mut ext_props = vec![VkExtensionProperties::default(); num_properties as _];
            let result = unsafe {
                self.commands.EnumerateDeviceExtensionProperties(
                    device,
                    ptr::null(),
                    &mut num_properties,
                    ext_props.as_mut_ptr() as _,
                )
            };

            if result == vk::SUCCESS {
                Ok(ext_props)
            } else {
                Err(result)
            }
        } else {
            Err(result)
        }
    }

    pub fn enum_physical_device_queue_family_properties(
        &self,
        device: vk::PhysicalDevice,
    ) -> Vec<vk::QueueFamilyProperties> {
        let mut num_properties: u32 = 0;
        unsafe {
            self.commands.GetPhysicalDeviceQueueFamilyProperties(
                device,
                &mut num_properties,
                ptr::null_mut(),
            )
        };

        let mut properties = vec![
            unsafe {
                std::mem::MaybeUninit::<vk::QueueFamilyProperties>::uninit().assume_init()
            };
            num_properties as _
        ];

        unsafe {
            self.commands.GetPhysicalDeviceQueueFamilyProperties(
                device,
                &mut num_properties,
                properties.as_mut_ptr(),
            )
        };

        properties
    }

    pub fn get_physical_device_features(
        &self,
        device: vk::PhysicalDevice,
    ) -> vk::PhysicalDeviceFeatures {
        let mut features = std::mem::MaybeUninit::<vk::PhysicalDeviceFeatures>::uninit();
        unsafe {
            self.commands
                .GetPhysicalDeviceFeatures(device, features.as_mut_ptr());
            features.assume_init()
        }
    }
    pub fn get_physical_device_properties(
        &self,
        device: vk::PhysicalDevice,
    ) -> vk::PhysicalDeviceProperties {
        let mut properties = std::mem::MaybeUninit::<vk::PhysicalDeviceProperties>::uninit();
        unsafe {
            self.commands
                .GetPhysicalDeviceProperties(device, properties.as_mut_ptr());
            properties.assume_init()
        }
    }

    pub fn get_physical_device_memory_properties(
        &self,
        device: vk::PhysicalDevice,
    ) -> vk::PhysicalDeviceMemoryProperties {
        let mut properties = std::mem::MaybeUninit::<vk::PhysicalDeviceMemoryProperties>::uninit();
        unsafe {
            self.commands
                .GetPhysicalDeviceMemoryProperties(device, properties.as_mut_ptr());
            properties.assume_init()
        }
    }
}

#[repr(transparent)]
pub struct VkPhysicalDeviceProperties(pub vk::PhysicalDeviceProperties);

impl fmt::Display for VkPhysicalDeviceProperties {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "name: {}, type: ", string_from_c_str(&self.0.deviceName))?;
        match self.0.deviceType {
            vk::PHYSICAL_DEVICE_TYPE_OTHER => write!(f, "other")?,
            vk::PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU => write!(f, "integrated")?,
            vk::PHYSICAL_DEVICE_TYPE_DISCRETE_GPU => write!(f, "discrete")?,
            vk::PHYSICAL_DEVICE_TYPE_VIRTUAL_GPU => write!(f, "virtual")?,
            vk::PHYSICAL_DEVICE_TYPE_CPU => write!(f, "cpu")?,
            _ => panic!("bad device type"),
        }
        Ok(())
    }
}

#[repr(transparent)]
pub struct VkQueueFamilyProperties(pub vk::QueueFamilyProperties);

impl fmt::Display for VkQueueFamilyProperties {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "num_queues: {}, flags: ", self.0.queueCount)?;
        if self.0.queueFlags & vk::QUEUE_GRAPHICS_BIT != 0 {
            write!(f, "graphics|")?;
        }
        if self.0.queueFlags & vk::QUEUE_COMPUTE_BIT != 0 {
            write!(f, "compute|")?;
        }
        if self.0.queueFlags & vk::QUEUE_TRANSFER_BIT != 0 {
            write!(f, "transfer|")?;
        }
        if self.0.queueFlags & vk::QUEUE_SPARSE_BINDING_BIT != 0 {
            write!(f, "sparse_binding|")?;
        }
        if self.0.queueFlags & vk::QUEUE_PROTECTED_BIT != 0 {
            write!(f, "protected|")?;
        }
        Ok(())
    }
}

#[repr(transparent)]
pub struct VkMemoryHeap(pub vk::MemoryHeap);

impl fmt::Display for VkMemoryHeap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "size: {number:>width$}, flags: ",
            number = self.0.size,
            width = 12,
        )?;
        if self.0.flags & vk::MEMORY_HEAP_DEVICE_LOCAL_BIT != 0 {
            write!(f, "device_local|")?;
        }
        if self.0.flags & vk::MEMORY_HEAP_MULTI_INSTANCE_BIT != 0 {
            write!(f, "multi_instance|")?;
        }
        Ok(())
    }
}

#[repr(transparent)]
pub struct VkMemoryType(pub vk::MemoryType);

impl fmt::Display for VkMemoryType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "heap_index: {}, flags: ", self.0.heapIndex)?;
        if self.0.propertyFlags & vk::MEMORY_PROPERTY_DEVICE_LOCAL_BIT != 0 {
            write!(f, "device_local|")?;
        }
        if self.0.propertyFlags & vk::MEMORY_PROPERTY_HOST_VISIBLE_BIT != 0 {
            write!(f, "host_visible|")?;
        }
        if self.0.propertyFlags & vk::MEMORY_PROPERTY_HOST_COHERENT_BIT != 0 {
            write!(f, "host_coherent|")?;
        }
        if self.0.propertyFlags & vk::MEMORY_PROPERTY_HOST_CACHED_BIT != 0 {
            write!(f, "host_cached|")?;
        }
        if self.0.propertyFlags & vk::MEMORY_PROPERTY_LAZILY_ALLOCATED_BIT != 0 {
            write!(f, "lazily_allocated|")?;
        }
        if self.0.propertyFlags & vk::MEMORY_PROPERTY_PROTECTED_BIT != 0 {
            write!(f, "protected|")?;
        }
        Ok(())
    }
}

pub struct Device {
    pub device: vk::Device,
    pub commands: vk::DeviceCommands,
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { self.commands.DestroyDevice(self.device, ptr::null()) };
    }
}

impl Device {
    pub fn new(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        extensions: &[&str],
        queues: &[(u32, u32)], // family index, queue count
        features: Option<vk::PhysicalDeviceFeatures>,
    ) -> Result<Device, vk::Result> {
        let max_queues = queues.iter().fold(0u32, |max, (_, x)| *x.max(&max));
        let priorities = vec![1.0; max_queues as usize];
        let queue_infos: Vec<_> = queues
            .iter()
            .map(|(family_index, count)| vk::DeviceQueueCreateInfo {
                sType: vk::STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
                pNext: ptr::null(),
                flags: 0,
                queueFamilyIndex: *family_index,
                queueCount: *count,
                pQueuePriorities: priorities.as_ptr(),
            })
            .collect();

        let extensions_cstr: Vec<_> = extensions
            .iter()
            .map(|&s| CString::new(s).unwrap())
            .collect();

        let extensions_ptr: Vec<_> = extensions_cstr.iter().map(|s| s.as_ptr()).collect();

        let info = vk::DeviceCreateInfo {
            sType: vk::STRUCTURE_TYPE_DEVICE_CREATE_INFO,
            pNext: ptr::null(),
            flags: 0,
            queueCreateInfoCount: queue_infos.len() as _,
            pQueueCreateInfos: queue_infos.as_ptr(),
            enabledLayerCount: 0,
            ppEnabledLayerNames: ptr::null(),
            enabledExtensionCount: extensions_ptr.len() as _,
            ppEnabledExtensionNames: extensions_ptr.as_ptr(),
            pEnabledFeatures: match features {
                Some(f) => &f,
                _ => ptr::null(),
            },
        };

        let mut device: vk::Device = 0;
        let result = unsafe {
            instance
                .commands
                .CreateDevice(physical_device, &info, ptr::null(), &mut device)
        };

        match result {
            vk::SUCCESS => {
                let commands = vk::DeviceCommands::new(instance.commands.GetDeviceProcAddr, device);
                Ok(Device { device, commands })
            }
            _ => Err(result),
        }
    }
}

fn main() {
    let vulkan = Vulkan::new().unwrap();
    let extensions = vulkan.enum_extensions().unwrap();
    let layers = vulkan.enum_layers().unwrap();

    println!("extensions:");
    for e in &extensions {
        println!("  {} {}", e.extension_name(), e.0.specVersion);
    }
    println!("layers:");
    for l in &layers {
        println!(
            "  {} ({}, {}): {}",
            l.layer_name(),
            l.0.specVersion,
            l.0.implementationVersion,
            l.description()
        );
    }

    let instance = Instance::new(
        vulkan,
        "app",
        "engine",
        &["VK_LAYER_KHRONOS_validation"],
        &["VK_EXT_debug_utils", "VK_KHR_surface"],
    )
    .unwrap();
    let physical_devices = instance.enum_physical_devices().unwrap();
    for &d in &physical_devices {
        let properties = instance.get_physical_device_properties(d);
        //let features = instance.get_physical_device_features(d);
        let queue_family_props = instance.enum_physical_device_queue_family_properties(d);

        let ver = vk::get_version(properties.apiVersion);
        println!(
            "device: {} ({},{},{})",
            VkPhysicalDeviceProperties(properties),
            ver.0,
            ver.1,
            ver.2
        );
        println!("  queue families:");
        for &q in &queue_family_props {
            println!("    {}", VkQueueFamilyProperties(q));
        }

        let mem_properties = instance.get_physical_device_memory_properties(d);

        println!("  memory_types:");
        for i in 0..mem_properties.memoryTypeCount {
            println!(
                "    {}",
                VkMemoryType(mem_properties.memoryTypes[i as usize])
            );
        }
        println!("  memory_heaps:");
        for i in 0..mem_properties.memoryHeapCount {
            println!(
                "    {}",
                VkMemoryHeap(mem_properties.memoryHeaps[i as usize])
            );
        }

        println!("  extensions:");
        let extensions = instance.enum_physical_device_extensions(d).unwrap();
        for e in &extensions {
            println!("    {}", e.extension_name());
        }
    }

    let physical_device = physical_devices[0];
    let queue_family_props = instance.enum_physical_device_queue_family_properties(physical_device);
    let queues: Vec<_> = queue_family_props
        .iter()
        .enumerate()
        .map(|(i, q)| (i as u32, q.queueCount))
        .collect();

    let device = Device::new(
        &instance,
        physical_device,
        &["VK_KHR_swapchain"],
        &queues,
        None,
    )
    .unwrap();

    let graphics_queue = queue_family_props
        .iter()
        .enumerate()
        .find(|(_, q)| q.queueFlags & vk::QUEUE_GRAPHICS_BIT != 0)
        .unwrap();
    let mut queue: vk::Queue = 0;
    unsafe {
        device
            .commands
            .GetDeviceQueue(device.device, graphics_queue.0 as _, 0, &mut queue);
    }

    unsafe {
        device.commands.QueueWaitIdle(queue);
    }

    println!("done");
}
