#!/bin/bash
set -e

CARGO_DIR="${CARGO_HOME:-$HOME/.cargo}"
RENDERERS=$(find "$CARGO_DIR/registry/src" -name "directx_renderer.rs" 2>/dev/null)

if [ -z "$RENDERERS" ]; then
    echo "No directx_renderer.rs found in $CARGO_DIR/registry/src. Run 'cargo fetch' first."
    exit 0
fi

for RENDERER in $RENDERERS; do
    python3 - "$RENDERER" << 'PYEOF'
import sys

renderer = sys.argv[1]
with open(renderer, 'r') as f:
    content = f.read()

if "ALPHA_CORRECTION_HLSL" in content:
    print(f"Already patched: {renderer}")
    sys.exit(0)

start_marker = "pub(crate) mod shader_resources {"
end_marker = "mod nvidia {"

start_idx = content.find(start_marker)
end_idx = content.find(end_marker)

if start_idx == -1 or end_idx == -1:
    print(f"Could not find shader_resources block in {renderer}")
    sys.exit(1)

replacement = '''pub(crate) mod shader_resources {
    use anyhow::Result;

    use windows::{
        Win32::Graphics::Direct3D::{
            Fxc::{D3DCOMPILE_DEBUG, D3DCOMPILE_SKIP_OPTIMIZATION, D3DCompile},
            ID3DBlob,
        },
        core::PCSTR,
    };

    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub(crate) enum ShaderModule {
        Quad,
        Shadow,
        Underline,
        PathRasterization,
        PathSprite,
        MonochromeSprite,
        PolychromeSprite,
        EmojiRasterization,
    }

    impl ShaderModule {
        pub fn as_str(&self) -> &str {
            match self {
                ShaderModule::Quad => "quad",
                ShaderModule::Shadow => "shadow",
                ShaderModule::Underline => "underline",
                ShaderModule::PathRasterization => "path_rasterization",
                ShaderModule::PathSprite => "path_sprite",
                ShaderModule::MonochromeSprite => "monochrome_sprite",
                ShaderModule::PolychromeSprite => "polychrome_sprite",
                ShaderModule::EmojiRasterization => "emoji_rasterization",
            }
        }
    }

    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub(crate) enum ShaderTarget {
        Vertex,
        Fragment,
    }

    pub(crate) struct RawShaderBytes<'t> {
        inner: &'t [u8],
        _blob: ID3DBlob,
    }

    impl<'t> RawShaderBytes<'t> {
        pub(crate) fn new(module: ShaderModule, target: ShaderTarget) -> Result<Self> {
            let blob = build_shader_blob(module, target)?;
            let inner = unsafe {
                std::slice::from_raw_parts(
                    blob.GetBufferPointer() as *const u8,
                    blob.GetBufferSize(),
                )
            };
            Ok(Self { inner, _blob: blob })
        }

        pub(crate) fn as_bytes(&'t self) -> &'t [u8] {
            self.inner
        }
    }

    pub(super) fn build_shader_blob(entry: ShaderModule, target: ShaderTarget) -> Result<ID3DBlob> {
        unsafe {
            const ALPHA_CORRECTION_HLSL: &str = include_str!("alpha_correction.hlsl");
            const SHADERS_HLSL: &str = include_str!("shaders.hlsl");
            const COLOR_TEXT_HLSL: &str = include_str!("color_text_raster.hlsl");

            let shader_source = if matches!(entry, ShaderModule::EmojiRasterization) {
                COLOR_TEXT_HLSL.replace(r#"#include "alpha_correction.hlsl""#, ALPHA_CORRECTION_HLSL)
            } else {
                SHADERS_HLSL.replace(r#"#include "alpha_correction.hlsl""#, ALPHA_CORRECTION_HLSL)
            };

            let entry_name = format!(
                "{}_{}\\0",
                entry.as_str(),
                match target {
                    ShaderTarget::Vertex => "vertex",
                    ShaderTarget::Fragment => "fragment",
                }
            );
            let target_profile = match target {
                ShaderTarget::Vertex => "vs_4_1\\0",
                ShaderTarget::Fragment => "ps_4_1\\0",
            };

            let mut compile_blob = None;
            let mut error_blob = None;

            let entry_point = PCSTR::from_raw(entry_name.as_ptr());
            let target_cstr = PCSTR::from_raw(target_profile.as_ptr());

            let ret = D3DCompile(
                shader_source.as_ptr() as *const _,
                shader_source.len(),
                windows_core::PCSTR::null(),
                None,
                None,
                entry_point,
                target_cstr,
                D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION,
                0,
                &mut compile_blob,
                Some(&mut error_blob),
            );
            if ret.is_err() {
                let Some(error_blob) = error_blob else {
                    return Err(anyhow::anyhow!("{ret:?}"));
                };

                let error_string =
                    std::ffi::CStr::from_ptr(error_blob.GetBufferPointer() as *const i8)
                        .to_string_lossy();
                log::error!("Shader compile error: {}", error_string);
                return Err(anyhow::anyhow!("Compile error: {}", error_string));
            }
            Ok(compile_blob.unwrap())
        }
    }
}

'''

content = content[:start_idx] + replacement + content[end_idx:]

with open(renderer, 'w') as f:
    f.write(content)

print(f"Patched: {renderer}")
PYEOF
done
