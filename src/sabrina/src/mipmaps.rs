// Copyright (c) 2026 Luka Dežulović
// SPDX-License-Identifier: MIT
//! Shared glTF texture preparation. PNG/JPEG loading supplies only level zero;
//! Embedded glTF images need mip levels for stable distant texture sampling.
use bevy::{
    gltf::GltfMaterial,
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{TextureDimension, TextureFormat},
};
use std::collections::HashMap;

pub fn prepare(
    library: &crate::assets::Library,
    gltfs: &Assets<Gltf>,
    materials: &Assets<GltfMaterial>,
    images: &mut Assets<Image>,
) {
    let mut textures = HashMap::new();
    for gltf in library.handles.iter().filter_map(|h| gltfs.get(h)) {
        for material in gltf.materials.iter().filter_map(|h| materials.get(h)) {
            for handle in [
                &material.base_color_texture,
                &material.emissive_texture,
                &material.metallic_roughness_texture,
                &material.occlusion_texture,
            ]
            .into_iter()
            .flatten()
            {
                textures.entry(handle.id()).or_insert(false);
            }
            if let Some(handle) = &material.normal_map_texture {
                textures.insert(handle.id(), true);
            }
        }
    }
    let mut count = 0;
    for (handle, normal) in textures {
        if let Some(mut image) = images.get_mut(handle) {
            count += usize::from(generate(&mut image, normal));
        }
    }
    println!("RENDER TEXTURES: generated shared mip chains for {count} material textures");
}

fn generate(image: &mut Image, normal: bool) -> bool {
    let texture = &mut image.texture_descriptor;
    if texture.mip_level_count != 1
        || texture.dimension != TextureDimension::D2
        || texture.size.depth_or_array_layers != 1
        || !matches!(
            texture.format,
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
        )
    {
        return false;
    }
    let Some(data) = image.data.as_mut() else {
        return false;
    };
    let srgb = texture.format == TextureFormat::Rgba8UnormSrgb;
    let (mut width, mut height) = (texture.size.width as usize, texture.size.height as usize);
    if width == 0 || height == 0 || data.len() != width * height * 4 {
        return false;
    }
    let linear: [f32; 256] = std::array::from_fn(|i| {
        let x = i as f32 / 255.0;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    });
    let mut offset = 0;
    while width > 1 || height > 1 {
        let (next_w, next_h) = ((width / 2).max(1), (height / 2).max(1));
        let next_offset = data.len();
        for y in 0..next_h {
            for x in 0..next_w {
                let mut sum = [0.0; 4];
                let mut samples = 0.0;
                // Proportional ranges keep the last row/column of odd-sized maps.
                for sy in y * height / next_h..(y + 1) * height / next_h {
                    for sx in x * width / next_w..(x + 1) * width / next_w {
                        let pixel = &data[offset + (sy * width + sx) * 4..][..4];
                        for channel in 0..4 {
                            sum[channel] += if srgb && channel < 3 {
                                linear[pixel[channel] as usize]
                            } else {
                                pixel[channel] as f32 / 255.0
                            };
                        }
                        samples += 1.0;
                    }
                }
                for value in &mut sum {
                    *value /= samples;
                }
                if normal {
                    let direction = (Vec3::from_array([sum[0], sum[1], sum[2]]) * 2.0 - Vec3::ONE)
                        .try_normalize()
                        .unwrap_or(Vec3::Z)
                        * 0.5
                        + Vec3::splat(0.5);
                    sum[..3].copy_from_slice(&direction.to_array());
                }
                for (channel, value) in sum.iter().enumerate() {
                    let value = if srgb && channel < 3 {
                        if *value <= 0.0031308 {
                            value * 12.92
                        } else {
                            1.055 * value.powf(1.0 / 2.4) - 0.055
                        }
                    } else {
                        *value
                    };
                    data.push((value.clamp(0.0, 1.0) * 255.0).round() as u8);
                }
            }
        }
        offset = next_offset;
        width = next_w;
        height = next_h;
        texture.mip_level_count += 1;
    }
    let mut sampler = match &image.sampler {
        ImageSampler::Descriptor(sampler) => sampler.clone(),
        ImageSampler::Default => ImageSamplerDescriptor::linear(),
    };
    sampler.mag_filter = ImageFilterMode::Linear;
    sampler.min_filter = ImageFilterMode::Linear;
    sampler.mipmap_filter = ImageFilterMode::Linear;
    sampler.anisotropy_clamp = 4;
    image.sampler = ImageSampler::Descriptor(sampler);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::RenderAssetUsages, render::render_resource::Extent3d};
    fn texture(format: TextureFormat, pixels: Vec<u8>, width: u32, height: u32) -> Image {
        Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            format,
            RenderAssetUsages::default(),
        )
    }
    #[test]
    fn color_mips_average_in_linear_light_and_preserve_base() {
        let original = vec![
            0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
        ];
        let mut image = texture(TextureFormat::Rgba8UnormSrgb, original.clone(), 2, 2);
        assert!(generate(&mut image, false));
        let data = image.data.as_ref().unwrap();
        assert_eq!(&data[..16], &original);
        assert_eq!(&data[16..], &[188, 188, 188, 255]);
        assert_eq!(image.texture_descriptor.mip_level_count, 2);
        assert!(!generate(&mut image, false));
    }
    #[test]
    fn odd_size_chain_ends_at_one_pixel_and_keeps_normals_unit_length() {
        let mut pixels = Vec::new();
        for i in 0..15 {
            pixels.extend_from_slice(if i % 2 == 0 {
                &[255, 128, 128, 255]
            } else {
                &[128, 128, 255, 255]
            });
        }
        let mut image = texture(TextureFormat::Rgba8Unorm, pixels, 5, 3);
        assert!(generate(&mut image, true));
        assert_eq!(image.texture_descriptor.mip_level_count, 3);
        let data = image.data.as_ref().unwrap();
        assert_eq!(data.len(), (15 + 2 + 1) * 4);
        for p in data[60..].chunks_exact(4) {
            let normal = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32) / 255.0 * 2.0 - Vec3::ONE;
            assert!((normal.length() - 1.0).abs() < 0.01);
        }
    }
}
