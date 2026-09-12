//! The real-window and smoke-test paths share this Vello output contract.
pub fn descriptor(width: u32, height: u32) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some("Vello storage output"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    }
}
pub fn create(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&descriptor(width, height))
        .create_view(&Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_descriptor_matches_vello_contract_and_resizes() {
        let d = descriptor(321, 123);
        assert_eq!(d.format, wgpu::TextureFormat::Rgba8Unorm);
        assert!(d
            .usage
            .contains(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING));
        assert_eq!((d.size.width, d.size.height), (321, 123));
        assert_eq!(descriptor(0, 0).size.width, 1);
    }
    #[test]
    #[ignore = "requires software Vulkan; run in the dedicated GPU CI job"]
    fn gpu_storage_and_blit_contract() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: true,
        }))
        .expect("software Vulkan adapter");
        eprintln!("GPU contract adapter: {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let output = create(&device, 32, 24);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Vello output contract"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                count: None,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
            }],
        });
        let _binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&output),
            }],
        });
        let surface_like = device
            .create_texture(&wgpu::TextureDescriptor {
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                ..descriptor(32, 24)
            })
            .create_view(&Default::default());
        let blitter = wgpu::util::TextureBlitter::new(&device, wgpu::TextureFormat::Bgra8Unorm);
        let mut encoder = device.create_command_encoder(&Default::default());
        blitter.copy(&device, &mut encoder, &output, &surface_like);
        queue.submit([encoder.finish()]);
        let error = pollster::block_on(scope.pop());
        assert!(
            error.is_none(),
            "production output / presentation contract failed: {error:?}"
        );
    }
}
