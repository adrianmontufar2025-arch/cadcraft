//! With multisampling the canvas draws smooth edges: a slanted edge drawn into a render pass of
//! four samples per pixel comes out with grey pixels along it, not a black-and-white staircase.
//! Renders through the canvas pipelines into an offscreen multisampled target, resolved and read
//! back.

use std::sync::Arc;

use cadcraft_engine::doc::Handle;
use cadcraft_engine::geom::{Bounds2, Vec2};
use cadcraft_engine::render::{DPrim, DisplayList, Kind};
use cadcraft_ui_egui::gpu::{self, CanvasCallback, GpuFaults, LAYER_DRAWING, MeshSlot, Stencil};
use egui_wgpu::wgpu;
use egui_wgpu::{CallbackTrait, ScreenDescriptor};

const SIZE: u32 = 64;
const SAMPLES: u32 = 4;

/// A black triangle with a slanted top edge, one world unit per pixel, world (32, 32) at the
/// centre.
fn callback() -> CanvasCallback {
    let pts = vec![Vec2::new(4.0, 4.0), Vec2::new(60.0, 4.0), Vec2::new(60.0, 40.0)];
    let bounds = Bounds2::from_points(pts.iter().copied());
    let prim =
        DPrim { handle: Handle(1), color: cadcraft_color::Rgb(0, 0, 0), aci7: false, lw: 0.0, transparency: 0, kind: Kind::Tris, start: 0, len: 3 };
    let list = DisplayList { prims: vec![prim], tris: pts, bounds, ..Default::default() };
    let slot: MeshSlot = Arc::default();
    *slot.lock().unwrap() = Some(gpu::build_mesh(&list, Vec2::ZERO, cadcraft_color::Rgb(255, 255, 255), false));
    CanvasCallback {
        key: 1,
        slot,
        offset: [-32.0, -32.0],
        scale: 1.0,
        center: [32.0, 32.0],
        layer: LAYER_DRAWING,
        xform: gpu::IDENTITY_XFORM,
        dash: 0.0,
        stencil: Stencil::Off,
    }
}

/// The resolved grey level (0 black … 255 white) of each pixel, row by row from the top.
fn render(rs: &egui_wgpu::RenderState, cb: &CanvasCallback) -> Vec<u8> {
    let (device, queue) = (&rs.device, &rs.queue);
    let extent = wgpu::Extent3d { width: SIZE, height: SIZE, depth_or_array_layers: 1 };
    let texture = |samples, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: extent,
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format: rs.target_format,
            usage,
            view_formats: &[],
        })
    };
    let msaa = texture(SAMPLES, wgpu::TextureUsages::RENDER_ATTACHMENT);
    let resolved = texture(1, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC);
    let (msaa_view, resolved_view) = (msaa.create_view(&Default::default()), resolved.create_view(&Default::default()));
    let screen = ScreenDescriptor { size_in_pixels: [SIZE, SIZE], pixels_per_point: 1.0 };
    let mut encoder = device.create_command_encoder(&Default::default());
    let mut renderer = rs.renderer.write();
    let _ = cb.prepare(device, queue, &screen, &mut encoder, &mut renderer.callback_resources);
    {
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &msaa_view,
                    resolve_target: Some(&resolved_view),
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Discard },
                    depth_slice: None,
                })],
                ..Default::default()
            })
            .forget_lifetime();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(SIZE as f32, SIZE as f32));
        let info = egui::PaintCallbackInfo { viewport: rect, clip_rect: rect, pixels_per_point: 1.0, screen_size_px: [SIZE, SIZE] };
        cb.paint(info, &mut pass, &renderer.callback_resources);
    }
    let row = SIZE * 4;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        resolved.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(SIZE) },
        },
        extent,
    );
    queue.submit([encoder.finish()]);
    buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(std::time::Duration::from_secs(10)) }).unwrap();
    let pixels = buffer.slice(..).get_mapped_range().unwrap();
    pixels.chunks(4).map(|p| ((u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])) / 3) as u8).collect()
}

#[test]
fn a_slanted_edge_is_drawn_smooth_with_multisampling() {
    let rs = egui_kittest::wgpu::create_render_state(egui_kittest::wgpu::default_wgpu_setup(), Default::default());
    let faults = Arc::new(GpuFaults::default());
    let target = gpu::install(&rs, &faults, None, SAMPLES);
    assert_eq!(target.samples, SAMPLES);
    let px = render(&rs, &callback());
    assert!(faults.take().is_none(), "the canvas pipelines draw into a pass of {SAMPLES} samples");
    let dark = px.iter().filter(|g| **g < 32).count();
    let grey = px.iter().filter(|g| (32..224).contains(*g)).count();
    assert!(dark > 500, "the triangle is drawn: {dark} dark pixels");
    assert!(grey >= 20, "its slanted edge has grey pixels, not a staircase: {grey}");
}

#[test]
fn the_window_asks_for_four_samples_unless_cadcraft_msaa_says_otherwise() {
    assert_eq!(gpu::msaa(None), 4);
    assert_eq!(gpu::msaa(Some("1")), 1);
    assert_eq!(gpu::msaa(Some(" 8 ")), 8);
    assert_eq!(gpu::msaa(Some("3")), 4);
    assert_eq!(gpu::msaa(Some("muchas")), 4);
}
