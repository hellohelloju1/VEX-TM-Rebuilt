//! Offline rasterization of the app's own egui output for layout review.
use super::*;
use std::collections::HashMap;
#[test]
fn render_reference_pages() {
    let Ok(dir) = std::env::var("TMRS_RENDER_DIR") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    for step in [0, 1, 2, 4, 5, 8, 9, 10] {
        let ctx = egui::Context::default();
        configure_style(&ctx);
        let mut w = wizard::Wizard::default();
        let mut app = App::default();
        app.load_demo("VRC_2025_PushBack");
        w.event = app.event;
        w.step = step;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440., 1000.),
            )),
            ..Default::default()
        };
        let out = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| w.ui(ui));
        });
        let nav = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t)
                    if t.galley.text().contains("Next") || t.galley.text().contains("Finish &") =>
                {
                    Some(t.pos.y + t.galley.size().y)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(!nav.is_empty(), "Navigation missing on step {step}");
        assert!(
            nav.iter().all(|y| *y < 995.),
            "Navigation offscreen on step {step}: {nav:?}"
        );
        raster(&ctx, out, 1440, 1000)
            .save(format!("{dir}/wizard-{}.png", step + 1))
            .unwrap();
    }
    let ctx = egui::Context::default();
    configure_style(&ctx);
    let mut app = App::default();
    app.load_demo("VRC_2025_PushBack");
    app.event.config.sponsors.push(Sponsor {
        name: "Program logo example".into(),
        image: include_bytes!("../assets/v5.png").to_vec(),
    });
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600., 1000.),
        )),
        time: Some(0.),
        ..Default::default()
    };
    let out = ctx.run(input, |ctx| {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::BLACK))
            .show(ctx, |ui| {
                displays::render(
                    ui,
                    &app.event,
                    &app.game(),
                    &app.event.config.displays[0],
                    0,
                    &app.timer,
                )
            });
    });
    raster(&ctx, out, 1600, 1000)
        .save(format!("{dir}/pit-display.png"))
        .unwrap();
}
fn raster(ctx: &egui::Context, out: egui::FullOutput, width: u32, height: u32) -> image::RgbaImage {
    let mut textures = HashMap::new();
    for (id, delta) in out.textures_delta.set {
        let egui::ImageData::Color(data) = delta.image;
        textures.insert(id, (*data).clone());
    }
    let mut image = image::RgbaImage::from_pixel(width, height, image::Rgba([245, 245, 245, 255]));
    for p in ctx.tessellate(out.shapes, 1.) {
        let egui::epaint::Primitive::Mesh(mesh) = p.primitive else {
            continue;
        };
        let Some(tex) = textures.get(&mesh.texture_id) else {
            continue;
        };
        for ids in mesh.indices.chunks_exact(3) {
            let a = mesh.vertices[ids[0] as usize];
            let b = mesh.vertices[ids[1] as usize];
            let c = mesh.vertices[ids[2] as usize];
            let minx = a
                .pos
                .x
                .min(b.pos.x)
                .min(c.pos.x)
                .max(p.clip_rect.min.x)
                .max(0.)
                .floor() as u32;
            let maxx = a
                .pos
                .x
                .max(b.pos.x)
                .max(c.pos.x)
                .min(p.clip_rect.max.x)
                .min(width as f32)
                .ceil() as u32;
            let miny = a
                .pos
                .y
                .min(b.pos.y)
                .min(c.pos.y)
                .max(p.clip_rect.min.y)
                .max(0.)
                .floor() as u32;
            let maxy = a
                .pos
                .y
                .max(b.pos.y)
                .max(c.pos.y)
                .min(p.clip_rect.max.y)
                .min(height as f32)
                .ceil() as u32;
            let cross = |a: egui::Pos2, b: egui::Pos2, c: egui::Pos2| {
                (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
            };
            let area = cross(a.pos, b.pos, c.pos);
            if area.abs() < 0.00001 {
                continue;
            }
            for y in miny..maxy {
                for x in minx..maxx {
                    let pos = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
                    let wa = cross(b.pos, c.pos, pos) / area;
                    let wb = cross(c.pos, a.pos, pos) / area;
                    let wc = 1. - wa - wb;
                    if wa < 0. || wb < 0. || wc < 0. {
                        continue;
                    }
                    let uv = a.uv.to_vec2() * wa + b.uv.to_vec2() * wb + c.uv.to_vec2() * wc;
                    let tx =
                        (uv.x * tex.size[0] as f32).clamp(0., tex.size[0] as f32 - 1.) as usize;
                    let ty =
                        (uv.y * tex.size[1] as f32).clamp(0., tex.size[1] as f32 - 1.) as usize;
                    let t = tex.pixels[ty * tex.size[0] + tx].to_array();
                    let ca = a.color.to_array();
                    let cb = b.color.to_array();
                    let cc = c.color.to_array();
                    let mut src = [0.; 4];
                    for i in 0..4 {
                        src[i] = (ca[i] as f32 * wa + cb[i] as f32 * wb + cc[i] as f32 * wc) / 255.
                            * t[i] as f32
                            / 255.;
                    }
                    let old = image.get_pixel(x, y).0;
                    let mut dst = [255; 4];
                    for i in 0..3 {
                        dst[i] =
                            (src[i] * 255. + old[i] as f32 * (1. - src[3])).clamp(0., 255.) as u8;
                    }
                    image.put_pixel(x, y, image::Rgba(dst));
                }
            }
        }
    }
    image
}
