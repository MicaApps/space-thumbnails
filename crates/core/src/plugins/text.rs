use crate::plugins::ThumbnailGenerator;
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use ab_glyph::{FontRef, PxScale};
use std::fs;
use std::path::Path;

pub struct TextGenerator;

impl ThumbnailGenerator for TextGenerator {
    fn name(&self) -> &str {
        "Text Renderer"
    }

    fn validate(&self, _header: &[u8], extension: &str) -> bool {
        matches!(
            extension,
            "txt" | "rs" | "json" | "toml" | "md" | "xml" | "log" | "ini" | "cfg" | "yaml" | "yml"
        )
    }

    fn generate(
        &self,
        buffer: Option<&[u8]>,
        width: u32,
        height: u32,
        _extension: &str,
        filepath: Option<&Path>,
    ) -> Result<Vec<u8>, String> {
        let file_buf;
        let text_data = if let Some(b) = buffer {
            b
        } else if let Some(path) = filepath {
            file_buf = fs::read(path).map_err(|e| e.to_string())?;
            &file_buf
        } else {
            return Err("No buffer or filepath provided for Text".to_string());
        };

        // 尝试解析为 UTF-8，替换无效字符
        let text = String::from_utf8_lossy(text_data);

        // 1. 创建透明背景图像
        let mut image = RgbaImage::new(width, height);

        // 2. 绘制文本
        // 不再绘制“纸张”背景，直接在透明背景上绘制文本
        // 为了适应文件图标的中间区域，我们需要计算文本的绘制范围
        // 假设图标中间有效区域大概是 50% ~ 60%
        let margin_x = (width as f32 * 0.25) as i32;
        let margin_y = (height as f32 * 0.3) as i32;
        let content_width = width as i32 - 2 * margin_x;
        let content_height = height as i32 - 2 * margin_y;

        // 3. 加载字体
        // 尝试加载常见的 Windows 字体
        let font_paths = [
            "C:\\Windows\\Fonts\\consola.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\segoeui.ttf",
        ];

        let mut font_data = Vec::new();
        for path in font_paths {
            if let Ok(data) = fs::read(path) {
                font_data = data;
                break;
            }
        }

        if font_data.is_empty() {
            return Ok(image.into_raw());
        }

        let font = FontRef::try_from_slice(&font_data).map_err(|_| "Error constructing font")?;

        // 4. 绘制文本
        // 缩小字体以适应图标
        let scale = PxScale::from(10.0); 
        let text_color = Rgba([50, 50, 50, 255]); // 深灰色文本，避免纯黑太突兀
        let line_height = 12;
        let max_lines = (content_height / line_height) - 1;
        let max_chars_per_line = (content_width / 6) as usize; 

        let mut y = margin_y;
        let x = margin_x;

        for (i, line) in text.lines().enumerate() {
            if i as i32 >= max_lines {
                break;
            }

            let truncated_line = if line.len() > max_chars_per_line {
                &line[..max_chars_per_line]
            } else {
                line
            };

            draw_text_mut(
                &mut image,
                text_color,
                x,
                y,
                scale,
                &font,
                truncated_line,
            );
            y += line_height;
        }

        Ok(image.into_raw())
    }
}
