# vulkan-bindings

Vulkan bindings for Rust generated from Khronos spec file (vk.xml)
Macro used to generate commands is inspired by the `vk-sys` package, part of `vulkano` (until [switching their low-level bindings to `ash`](https://github.com/vulkano-rs/vulkano/issues/1500)).

The bindings are generated directly from vk.xml and therefore up to date with recent
Vulkan core specification. At this time, only platform independent extensions are emitted.

For information on how to use it, see examples.

