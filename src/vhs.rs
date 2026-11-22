//! VHS tape look (vhs_mpalko, two passes), ported from metroidvania3D-rust. The frame is drawn
//! into an off-screen buffer, then pass 0 shrinks it to VHS resolution in YIQ and pass 1 scales
//! it back to the window, with CRT scanlines. F3 or `--no-vhs` turns it off.

use raylib::ffi;
use std::ffi::{CString, c_void};

/// 1 matches the original shader; lower is softer.
const VHS_INTENSITY: f32 = 0.7;
/// TV lines over the whole image (240, as in metroidvania3D-rust) and how much darker the gaps get.
const SCANLINES: f32 = 240.0;
const SCANLINE_STRENGTH: f32 = 0.25;

pub struct Vhs {
    pub enabled: bool,
    pass: [ffi::Shader; 2],
    scene: ffi::RenderTexture2D, // the frame, as the game draws it
    yiq: ffi::RenderTexture2D,   // pass 0 output: float, because I and Q go negative
    size: (i32, i32),
}

fn set_f(shader: ffi::Shader, name: &str, v: &[f32], kind: ffi::ShaderUniformDataType) {
    let name = CString::new(name).unwrap();
    unsafe {
        let loc = ffi::GetShaderLocation(shader, name.as_ptr());
        ffi::SetShaderValue(shader, loc, v.as_ptr() as *const c_void, kind as i32);
    }
}

fn load_pass(fs: &str) -> ffi::Shader {
    let fs = CString::new(fs).unwrap();
    let shader = unsafe { ffi::LoadShaderFromMemory(std::ptr::null(), fs.as_ptr()) };
    set_f(shader, "intensity", &[VHS_INTENSITY], ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT);
    shader
}

fn load_float_render_texture(width: i32, height: i32) -> ffi::RenderTexture2D {
    unsafe {
        let mut target: ffi::RenderTexture2D = std::mem::zeroed();
        target.id = ffi::rlLoadFramebuffer();
        let fmt = ffi::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R16G16B16A16 as i32;
        target.texture = ffi::Texture2D {
            id: ffi::rlLoadTexture(std::ptr::null(), width, height, fmt, 1),
            width,
            height,
            mipmaps: 1,
            format: fmt,
        };
        ffi::rlFramebufferAttach(
            target.id,
            target.texture.id,
            ffi::rlFramebufferAttachType::RL_ATTACHMENT_COLOR_CHANNEL0 as i32,
            ffi::rlFramebufferAttachTextureType::RL_ATTACHMENT_TEXTURE2D as i32,
            0,
        );
        if !ffi::rlFramebufferComplete(target.id) {
            eprintln!("VHS: the float framebuffer is not complete");
        }
        target
    }
}

impl Vhs {
    /// Call after the window exists.
    pub fn new(enabled: bool) -> Self {
        let vhs = Vhs {
            enabled,
            pass: [
                load_pass(include_str!("../assets/shaders/vhs_pass0.fs")),
                load_pass(include_str!("../assets/shaders/vhs_pass1.fs")),
            ],
            scene: unsafe { std::mem::zeroed() },
            yiq: unsafe { std::mem::zeroed() },
            size: (0, 0),
        };
        let flt = ffi::ShaderUniformDataType::SHADER_UNIFORM_FLOAT;
        set_f(vhs.pass[1], "lines", &[SCANLINES], flt);
        set_f(vhs.pass[1], "strength", &[SCANLINE_STRENGTH], flt);
        vhs
    }

    fn unload_targets(&mut self) {
        if self.scene.id != 0 {
            unsafe {
                ffi::UnloadRenderTexture(self.scene);
                ffi::UnloadRenderTexture(self.yiq);
            }
            self.scene.id = 0;
        }
    }

    /// The buffers follow the window, which is resizable.
    fn fit_window(&mut self) -> bool {
        let size = unsafe { (ffi::GetScreenWidth(), ffi::GetScreenHeight()) };
        if size.0 <= 0 || size.1 <= 0 {
            return false; // minimized
        }
        if size != self.size {
            self.unload_targets();
            let (w, h) = size;
            let bilinear = ffi::TextureFilter::TEXTURE_FILTER_BILINEAR as i32;
            unsafe {
                self.scene = ffi::LoadRenderTexture(w, h);
                self.yiq = load_float_render_texture(w, h);
                ffi::SetTextureFilter(self.scene.texture, bilinear);
                ffi::SetTextureFilter(self.yiq.texture, bilinear);
            }
            for s in self.pass {
                set_f(s, "outputSize", &[w as f32, h as f32], ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC2);
            }
            self.size = size;
        }
        true
    }

    /// Call right after BeginDrawing: what is drawn until `end` goes to the VHS buffer.
    /// Returns whether it did, to pass to `end`.
    pub fn begin(&mut self) -> bool {
        let on = self.enabled && self.fit_window();
        if on {
            unsafe { ffi::BeginTextureMode(self.scene) };
        }
        on
    }

    /// Runs both passes, the last one straight onto the window.
    pub fn end(&self, began: bool) {
        if !began {
            return;
        }
        // Both passes read and write by gl_FragCoord without flipping, so no Y flip is needed.
        let white = ffi::Color { r: 255, g: 255, b: 255, a: 255 };
        unsafe {
            ffi::EndTextureMode();
            ffi::BeginTextureMode(self.yiq);
            ffi::BeginShaderMode(self.pass[0]);
            ffi::DrawTexture(self.scene.texture, 0, 0, white);
            ffi::EndShaderMode();
            ffi::EndTextureMode();
            ffi::BeginShaderMode(self.pass[1]);
            ffi::DrawTexture(self.yiq.texture, 0, 0, white);
            ffi::EndShaderMode();
        }
    }
}

impl Drop for Vhs {
    fn drop(&mut self) {
        self.unload_targets();
        for s in self.pass {
            unsafe { ffi::UnloadShader(s) };
        }
    }
}
