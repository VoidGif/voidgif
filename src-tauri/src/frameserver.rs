//! Serves frame images to the webview over the custom `voidframe` scheme.
//!
//! URL shape: voidframe://frame/{frame_id}?w={max_width}&r={cache_rev}
//! (surfaces as http://voidframe.localhost/frame/... in WebView2).
//! Responses are PNG with immutable caching — the frontend busts caches by
//! bumping the `r` query parameter whenever pixels change.

use crate::session::PixelRef;
use crate::state::AppState;
use fast_image_resize as fir;
use tauri::{AppHandle, Manager, Runtime, UriSchemeContext};

fn bgra_to_rgba(buf: &mut [u8]) {
    for px in buf.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
}

fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity((width * height) as usize);
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(rgba).map_err(|e| e.to_string())?;
        writer.finish().map_err(|e| e.to_string())?;
    }
    Ok(out)
}

fn downscale(rgba: Vec<u8>, pixels: PixelRef, max_w: u32) -> Result<(Vec<u8>, u32, u32), String> {
    if max_w == 0 || pixels.width <= max_w {
        return Ok((rgba, pixels.width, pixels.height));
    }
    let dst_w = max_w;
    let dst_h = ((pixels.height as u64 * max_w as u64) / pixels.width as u64).max(1) as u32;
    let src = fir::images::Image::from_vec_u8(pixels.width, pixels.height, rgba, fir::PixelType::U8x4)
        .map_err(|e| e.to_string())?;
    let mut dst = fir::images::Image::new(dst_w, dst_h, fir::PixelType::U8x4);
    let mut resizer = fir::Resizer::new();
    let options = fir::ResizeOptions::new()
        .resize_alg(fir::ResizeAlg::Convolution(fir::FilterType::Bilinear))
        .use_alpha(false);
    resizer
        .resize(&src, &mut dst, Some(&options))
        .map_err(|e| e.to_string())?;
    Ok((dst.into_vec(), dst_w, dst_h))
}

fn respond(status: u16, content_type: &str, body: Vec<u8>) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(status)
        .header("Content-Type", content_type)
        .header("Cache-Control", "public, max-age=31536000, immutable")
        .header("Access-Control-Allow-Origin", "*")
        .body(body)
        .expect("static response builder cannot fail")
}

fn serve_frame<R: Runtime>(app: &AppHandle<R>, frame_id: u64, max_w: u32) -> Result<tauri::http::Response<Vec<u8>>, String> {
    let state = app.state::<AppState>();
    let mut guard = state.session.lock().map_err(|_| "session lock poisoned")?;
    let session = guard.as_mut().ok_or("no active session")?;
    let frame = session
        .frame_by_id(frame_id)
        .ok_or_else(|| format!("unknown frame {frame_id}"))?;
    let mut bgra = session.read_pixels(frame.pixels).map_err(|e| e.to_string())?;
    drop(guard); // release the lock before CPU-heavy encode

    bgra_to_rgba(&mut bgra);
    let (rgba, w, h) = downscale(bgra, frame.pixels, max_w)?;
    let png = encode_png(&rgba, w, h)?;
    Ok(respond(200, "image/png", png))
}

pub fn handle<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let uri = request.uri();
    let path = uri.path();
    let query = uri.query().unwrap_or("");

    let Some(id_str) = path.strip_prefix("/frame/") else {
        return respond(404, "text/plain", b"not found".to_vec());
    };
    let Ok(frame_id) = id_str.parse::<u64>() else {
        return respond(400, "text/plain", b"bad frame id".to_vec());
    };
    let max_w = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("w="))
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    match serve_frame(ctx.app_handle(), frame_id, max_w) {
        Ok(resp) => resp,
        Err(e) => {
            log::warn!("frame server error: {e}");
            respond(500, "text/plain", e.into_bytes())
        }
    }
}
