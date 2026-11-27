//! Le rendu : des quadrilatères texturés, regroupés tant que la texture et
//! le mélange ne changent pas. Textures et couleurs en alpha prémultiplié.
//! Comme la PS2, l'image est dessinée au pixel près sur un écran de
//! 640 × 448 (y vers le bas), puis agrandie dans la fenêtre en 4:3.

use std::collections::HashMap;

use miniquad::*;

/// Les mélanges du GS : le mini-jeu n'emploie que le normal (alpha).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Blend {
    Normal,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

/// Sommets par lot (au plus).
const BATCH: usize = 4096;

pub struct Renderer {
    ctx: Box<dyn RenderingBackend>,
    shader: ShaderId,
    pipelines: HashMap<Blend, Pipeline>,
    bindings: Bindings,
    white: TextureId,
    vertices: Vec<Vertex>,
    texture: Option<TextureId>,
    blend: Blend,
    mvp: [f32; 16],
    /// L'écran de la PS2 : sa texture, sa passe, sa taille.
    screen: TextureId,
    pass: RenderPass,
    size: (f32, f32),
    viewport: (i32, i32, i32, i32),
}

impl Renderer {
    pub fn new(size: (f32, f32)) -> Renderer {
        let mut ctx = window::new_rendering_backend();
        let screen = ctx.new_render_texture(TextureParams { width: size.0 as u32, height: size.1 as u32, format: TextureFormat::RGBA8, ..Default::default() });
        ctx.texture_set_filter(screen, FilterMode::Linear, MipmapFilterMode::None);
        let pass = ctx.new_render_pass(screen, None);
        let meta = ShaderMeta { images: vec!["tex".to_string()], uniforms: UniformBlockLayout { uniforms: vec![UniformDesc::new("mvp", UniformType::Mat4)] } };
        let shader = ctx.new_shader(ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT }, meta).expect("shader");
        let white = ctx.new_texture_from_rgba8(1, 1, &[255; 4]);
        let vertex_buffer = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Vertex>(BATCH));
        // Deux triangles par quadrilatère.
        let indices: Vec<u16> = (0..(BATCH / 4) as u16).flat_map(|q| [0, 1, 2, 2, 1, 3].map(|i| q * 4 + i)).collect();
        let index_buffer = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices));
        let bindings = Bindings { vertex_buffers: vec![vertex_buffer], index_buffer, images: vec![white] };
        Renderer { ctx, shader, pipelines: HashMap::new(), bindings, white, vertices: Vec::with_capacity(BATCH), texture: None, blend: Blend::Normal, mvp: [0.0; 16], screen, pass, size, viewport: (0, 0, 1, 1) }
    }

    /// Une texture RGBA prémultipliée, lissée comme le filtrage bilinéaire
    /// du GS.
    pub fn texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> TextureId {
        let id = self.ctx.new_texture_from_rgba8(width as u16, height as u16, rgba);
        self.ctx.texture_set_filter(id, FilterMode::Linear, MipmapFilterMode::None);
        self.ctx.texture_set_wrap(id, TextureWrap::Clamp, TextureWrap::Clamp);
        id
    }

    /// Libère une texture.
    pub fn delete(&mut self, id: TextureId) {
        self.ctx.delete_texture(id);
    }

    /// Une texture répétée au-delà de ses bords (mode REPEAT du GS) plutôt
    /// que bloquée.
    pub fn repeat(&mut self, id: TextureId) {
        self.ctx.texture_set_wrap(id, TextureWrap::Repeat, TextureWrap::Repeat);
    }

    /// Commence une image sur l'écran de la PS2, qui occupera ensuite
    /// `viewport` (x, y depuis le bas, largeur, hauteur) de la fenêtre.
    pub fn begin(&mut self, viewport: (i32, i32, i32, i32)) {
        self.viewport = viewport;
        self.ctx.begin_pass(Some(self.pass), PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        let (w, h) = self.size;
        self.ctx.apply_viewport(0, 0, w as i32, h as i32);
        // L'écran est rangé ligne du haut d'abord (y = 0 en v = 0), relu
        // tel quel à l'agrandissement.
        let (sx, sy) = (2.0 / w, 2.0 / h);
        self.mvp = [sx, 0.0, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0];
        self.texture = None;
    }

    /// L'écran dessiné, agrandi dans la fenêtre (bandes noires autour).
    pub fn finish(&mut self) {
        self.flush();
        self.ctx.end_render_pass();
        self.ctx.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        let (x, y, w, h) = self.viewport;
        self.ctx.apply_viewport(x, y, w, h);
        let (sw, sh) = self.size;
        self.mvp = [2.0 / sw, 0.0, 0.0, 0.0, 0.0, -2.0 / sh, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 1.0, 0.0, 1.0];
        let corners = [(0.0, 0.0), (sw, 0.0), (0.0, sh), (sw, sh)];
        self.quad(Some(self.screen), corners, [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)], [1.0; 4], Blend::Normal);
        self.flush();
    }

    pub fn commit(&mut self) {
        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }

    /// Un quadrilatère : coins (haut gauche, haut droite, bas gauche, bas
    /// droite) en pixels de l'écran, leurs coordonnées de texture (0..1),
    /// la couleur prémultipliée. Sans texture : couleur unie.
    pub fn quad(&mut self, texture: Option<TextureId>, corners: [(f32, f32); 4], uv: [(f32, f32); 4], color: [f32; 4], blend: Blend) {
        let texture = texture.unwrap_or(self.white);
        if self.texture != Some(texture) || self.blend != blend || self.vertices.len() + 4 > BATCH {
            self.flush();
            self.texture = Some(texture);
            self.blend = blend;
        }
        for i in 0..4 {
            self.vertices.push(Vertex { pos: [corners[i].0, corners[i].1], uv: [uv[i].0, uv[i].1], color });
        }
    }

    fn flush(&mut self) {
        let Some(texture) = self.texture else { return };
        if self.vertices.is_empty() {
            return;
        }
        let pipeline = self.pipeline(self.blend);
        self.ctx.buffer_update(self.bindings.vertex_buffers[0], BufferSource::slice(&self.vertices));
        self.bindings.images[0] = texture;
        self.ctx.apply_pipeline(&pipeline);
        self.ctx.apply_bindings(&self.bindings);
        self.ctx.apply_uniforms(UniformsSource::table(&self.mvp));
        self.ctx.draw(0, (self.vertices.len() / 4 * 6) as i32, 1);
        self.vertices.clear();
    }

    fn pipeline(&mut self, blend: Blend) -> Pipeline {
        if let Some(&p) = self.pipelines.get(&blend) {
            return p;
        }
        let one = BlendFactor::One;
        let state = match blend {
            Blend::Normal => BlendState::new(Equation::Add, one, BlendFactor::OneMinusValue(BlendValue::SourceAlpha)),
        };
        let keep_alpha = BlendState::new(Equation::Add, BlendFactor::Zero, one);
        let params = PipelineParams { color_blend: Some(state), alpha_blend: Some(keep_alpha), ..Default::default() };
        let p = self.ctx.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("pos", VertexFormat::Float2), VertexAttribute::new("uv", VertexFormat::Float2), VertexAttribute::new("color", VertexFormat::Float4)],
            self.shader,
            params,
        );
        self.pipelines.insert(blend, p);
        p
    }

    /// Les pixels RGBA de la fenêtre (tests), ligne du haut d'abord.
    pub fn read_pixels(&mut self, width: i32, height: i32) -> Vec<u8> {
        self.flush();
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        // SAFETY : le tampon a la taille demandée, en RGBA 8 bits.
        unsafe {
            gl::glReadPixels(0, 0, width, height, gl::GL_RGBA, gl::GL_UNSIGNED_BYTE, pixels.as_mut_ptr().cast());
        }
        let row = (width * 4) as usize;
        let mut flipped = Vec::with_capacity(pixels.len());
        for line in pixels.chunks_exact(row).rev() {
            flipped.extend_from_slice(line);
        }
        flipped
    }
}

const VERTEX: &str = r"#version 100
attribute vec2 pos;
attribute vec2 uv;
attribute vec4 color;
uniform mat4 mvp;
varying vec2 v_uv;
varying vec4 v_color;
void main() {
    v_uv = uv;
    v_color = color;
    gl_Position = mvp * vec4(pos, 0.0, 1.0);
}
";

const FRAGMENT: &str = r"#version 100
precision mediump float;
varying vec2 v_uv;
varying vec4 v_color;
uniform sampler2D tex;
void main() {
    gl_FragColor = texture2D(tex, v_uv) * v_color;
}
";
