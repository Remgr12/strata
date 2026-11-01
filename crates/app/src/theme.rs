use gpui::{rgb, Rgba};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Core Material Design 3 tonal palette for Strata.
///
/// Default identity:
/// - Primary Body Tone (Base): Hazy Aerostat Blue-Gray
///   - Light Mode / Substrate: `#D4DFE6`
///   - Dark Mode: `#1A2229`
/// - Core Technical Blue (High-Legibility / Primary Mark): Steel Indigo — `#26466D`
/// - Low-Key Cut-Through (Physical Accents / Key CTAs): Soft Dried Apricot — `#E08A63`
/// - Data Highlight (Telemetry / Indicators): Faded Ice Cyan — `#6B9DA8`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePalette {
    pub name: String,
    pub is_dark: bool,

    // Core anchor colors
    pub substrate: u32,
    pub primary: u32,
    pub accent: u32,
    pub telemetry: u32,

    // Material 3 surface & elevation containers
    pub surface_lowest: u32,
    pub surface: u32,
    pub surface_container: u32,
    pub surface_container_high: u32,

    // Borders & outlines
    pub outline: u32,
    pub outline_variant: u32,

    // Interactive & tonal container states
    pub primary_container: u32,
    pub primary_hover: u32,
    pub on_primary: u32,

    pub accent_container: u32,
    pub accent_hover: u32,
    pub on_accent: u32,

    pub telemetry_container: u32,

    // Typography hierarchy
    pub text_primary: u32,
    pub text_secondary: u32,
    pub text_muted: u32,

    // Semantic error state
    pub danger: u32,
    pub danger_container: u32,
}

/// Human-editable JSON representation using `#RRGGBB` hex strings for custom theme files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomThemeConfig {
    pub name: String,
    pub is_dark: bool,
    pub substrate: String,
    pub primary: String,
    pub accent: String,
    pub telemetry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_primary: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeTokenField {
    Substrate,
    Primary,
    Accent,
    Telemetry,
    Surface,
    TextPrimary,
}

impl ThemeTokenField {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Substrate => "Substrate (Base)",
            Self::Primary => "Primary (Technical Blue)",
            Self::Accent => "Accent (Key CTA)",
            Self::Telemetry => "Telemetry (Data Highlight)",
            Self::Surface => "Surface Container",
            Self::TextPrimary => "Primary Text",
        }
    }
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self::aerostat_dark()
    }
}

impl ThemePalette {
    /// Default Dark Mode theme anchored on Hazy Aerostat Blue-Gray (`#1A2229`),
    /// Steel Indigo (`#26466D`), Soft Dried Apricot (`#E08A63`), and Faded Ice Cyan (`#6B9DA8`).
    pub fn aerostat_dark() -> Self {
        Self {
            name: "Aerostat Dark".to_string(),
            is_dark: true,
            substrate: 0x1A2229,
            primary: 0x26466D,
            accent: 0xE08A63,
            telemetry: 0x6B9DA8,

            surface_lowest: 0x141B20,
            surface: 0x1E272F,
            surface_container: 0x242F38,
            surface_container_high: 0x2C3944,

            outline: 0x31404D,
            outline_variant: 0x232E38,

            primary_container: 0x203856,
            primary_hover: 0x305582,
            on_primary: 0xEEF4F8,

            accent_container: 0x382822,
            accent_hover: 0xE89B78,
            on_accent: 0x1A2229,

            telemetry_container: 0x1F3238,

            text_primary: 0xE5EDF3,
            text_secondary: 0xA2B3C1,
            text_muted: 0x6C7F8E,

            danger: 0xD96B6B,
            danger_container: 0x362024,
        }
    }

    /// Default Light Mode theme anchored on Hazy Aerostat Blue-Gray Substrate (`#D4DFE6`),
    /// Steel Indigo (`#26466D`), Soft Dried Apricot (`#E08A63`), and Faded Ice Cyan (`#6B9DA8`).
    pub fn aerostat_light() -> Self {
        Self {
            name: "Aerostat Light".to_string(),
            is_dark: false,
            substrate: 0xD4DFE6,
            primary: 0x26466D,
            accent: 0xE08A63,
            telemetry: 0x6B9DA8,

            surface_lowest: 0xE1EAF0,
            surface: 0xCBD7DF,
            surface_container: 0xC1CFD9,
            surface_container_high: 0xB4C4D0,

            outline: 0xA6B8C5,
            outline_variant: 0xBDCDD8,

            primary_container: 0xBCCCE0,
            primary_hover: 0x1E3858,
            on_primary: 0xF5F9FC,

            accent_container: 0xDFC6BB,
            accent_hover: 0xD17952,
            on_accent: 0x1A2229,

            telemetry_container: 0xBDD1D7,

            text_primary: 0x1A2229,
            text_secondary: 0x384958,
            text_muted: 0x5B6E7E,

            danger: 0xB84444,
            danger_container: 0xDEC2C4,
        }
    }

    /// Recompute derived Material 3 tonal containers when core colors are customized.
    pub fn rebuild_derived_tokens(&mut self) {
        let luma = Self::relative_luma(self.substrate);
        self.is_dark = luma < 128;

        if self.is_dark {
            self.surface_lowest = Self::mix(self.substrate, 0x000000, 0.22);
            self.surface = Self::mix(self.substrate, 0xFFFFFF, 0.04);
            self.surface_container = Self::mix(self.substrate, 0xFFFFFF, 0.08);
            self.surface_container_high = Self::mix(self.substrate, 0xFFFFFF, 0.13);
            self.outline = Self::mix(self.substrate, 0xFFFFFF, 0.17);
            self.outline_variant = Self::mix(self.substrate, 0xFFFFFF, 0.07);

            self.primary_container = Self::mix(self.substrate, self.primary, 0.65);
            self.primary_hover = Self::mix(self.primary, 0xFFFFFF, 0.12);
            self.on_primary = 0xEEF4F8;

            self.accent_container = Self::mix(self.substrate, self.accent, 0.25);
            self.accent_hover = Self::mix(self.accent, 0xFFFFFF, 0.14);
            self.on_accent = if Self::relative_luma(self.accent) > 135 {
                0x1A2229
            } else {
                0xFFFFFF
            };

            self.telemetry_container = Self::mix(self.substrate, self.telemetry, 0.25);

            self.text_primary = 0xE5EDF3;
            self.text_secondary = Self::mix(self.text_primary, self.substrate, 0.32);
            self.text_muted = Self::mix(self.text_primary, self.substrate, 0.56);
            self.danger = 0xD96B6B;
            self.danger_container = Self::mix(self.substrate, 0xD96B6B, 0.22);
        } else {
            self.surface_lowest = Self::mix(self.substrate, 0xFFFFFF, 0.35);
            self.surface = Self::mix(self.substrate, 0x1A2229, 0.05);
            self.surface_container = Self::mix(self.substrate, 0x1A2229, 0.10);
            self.surface_container_high = Self::mix(self.substrate, 0x1A2229, 0.16);
            self.outline = Self::mix(self.substrate, 0x1A2229, 0.24);
            self.outline_variant = Self::mix(self.substrate, 0x1A2229, 0.12);

            self.primary_container = Self::mix(self.substrate, self.primary, 0.22);
            self.primary_hover = Self::mix(self.primary, 0x000000, 0.15);
            self.on_primary = 0xF5F9FC;

            self.accent_container = Self::mix(self.substrate, self.accent, 0.24);
            self.accent_hover = Self::mix(self.accent, 0x000000, 0.10);
            self.on_accent = if Self::relative_luma(self.accent) > 135 {
                0x1A2229
            } else {
                0xFFFFFF
            };

            self.telemetry_container = Self::mix(self.substrate, self.telemetry, 0.24);

            self.text_primary = 0x1A2229;
            self.text_secondary = Self::mix(self.text_primary, self.substrate, 0.22);
            self.text_muted = Self::mix(self.text_primary, self.substrate, 0.45);
            self.danger = 0xB84444;
            self.danger_container = Self::mix(self.substrate, 0xB84444, 0.20);
        }
    }

    pub fn get_field_hex(&self, field: ThemeTokenField) -> String {
        let val = match field {
            ThemeTokenField::Substrate => self.substrate,
            ThemeTokenField::Primary => self.primary,
            ThemeTokenField::Accent => self.accent,
            ThemeTokenField::Telemetry => self.telemetry,
            ThemeTokenField::Surface => self.surface,
            ThemeTokenField::TextPrimary => self.text_primary,
        };
        Self::format_hex(val)
    }

    pub fn set_field_hex(&mut self, field: ThemeTokenField, hex_str: &str) -> Result<(), String> {
        let parsed = Self::parse_hex(hex_str)?;
        match field {
            ThemeTokenField::Substrate => {
                self.substrate = parsed;
                self.rebuild_derived_tokens();
            }
            ThemeTokenField::Primary => {
                self.primary = parsed;
                self.rebuild_derived_tokens();
            }
            ThemeTokenField::Accent => {
                self.accent = parsed;
                self.rebuild_derived_tokens();
            }
            ThemeTokenField::Telemetry => {
                self.telemetry = parsed;
                self.rebuild_derived_tokens();
            }
            ThemeTokenField::Surface => {
                self.surface = parsed;
                self.surface_container = if self.is_dark {
                    Self::mix(parsed, 0xFFFFFF, 0.06)
                } else {
                    Self::mix(parsed, 0x1A2229, 0.06)
                };
            }
            ThemeTokenField::TextPrimary => {
                self.text_primary = parsed;
                self.text_secondary = Self::mix(parsed, self.substrate, 0.30);
                self.text_muted = Self::mix(parsed, self.substrate, 0.54);
            }
        }
        self.name = "Custom".to_string();
        Ok(())
    }

    pub fn to_custom_config(&self) -> CustomThemeConfig {
        CustomThemeConfig {
            name: self.name.clone(),
            is_dark: self.is_dark,
            substrate: Self::format_hex(self.substrate),
            primary: Self::format_hex(self.primary),
            accent: Self::format_hex(self.accent),
            telemetry: Self::format_hex(self.telemetry),
            surface: Some(Self::format_hex(self.surface)),
            text_primary: Some(Self::format_hex(self.text_primary)),
        }
    }

    pub fn from_custom_config(cfg: &CustomThemeConfig) -> Result<Self, String> {
        let mut palette = if cfg.is_dark {
            Self::aerostat_dark()
        } else {
            Self::aerostat_light()
        };
        palette.name = cfg.name.clone();
        palette.substrate = Self::parse_hex(&cfg.substrate)?;
        palette.primary = Self::parse_hex(&cfg.primary)?;
        palette.accent = Self::parse_hex(&cfg.accent)?;
        palette.telemetry = Self::parse_hex(&cfg.telemetry)?;
        palette.rebuild_derived_tokens();
        if let Some(ref surf) = cfg.surface {
            if let Ok(s) = Self::parse_hex(surf) {
                palette.surface = s;
            }
        }
        if let Some(ref txt) = cfg.text_primary {
            if let Ok(t) = Self::parse_hex(txt) {
                palette.text_primary = t;
            }
        }
        Ok(palette)
    }

    pub fn parse_hex(input: &str) -> Result<u32, String> {
        let trimmed = input.trim().trim_start_matches('#').trim_start_matches("0x");
        if trimmed.len() != 6 {
            return Err("Expected 6-digit hex color (e.g. #1A2229)".to_string());
        }
        u32::from_str_radix(trimmed, 16)
            .map_err(|_| format!("Invalid hex color '{}'", input))
    }

    pub fn format_hex(color: u32) -> String {
        format!("#{:06X}", color & 0xFFFFFF)
    }

    fn relative_luma(color: u32) -> u32 {
        let r = (color >> 16) & 0xFF;
        let g = (color >> 8) & 0xFF;
        let b = color & 0xFF;
        (r * 299 + g * 587 + b * 114) / 1000
    }

    fn mix(c1: u32, c2: u32, t: f32) -> u32 {
        let r1 = ((c1 >> 16) & 0xFF) as f32;
        let g1 = ((c1 >> 8) & 0xFF) as f32;
        let b1 = (c1 & 0xFF) as f32;

        let r2 = ((c2 >> 16) & 0xFF) as f32;
        let g2 = ((c2 >> 8) & 0xFF) as f32;
        let b2 = (c2 & 0xFF) as f32;

        let r = (r1 + (r2 - r1) * t).round().clamp(0.0, 255.0) as u32;
        let g = (g1 + (g2 - g1) * t).round().clamp(0.0, 255.0) as u32;
        let b = (b1 + (b2 - b1) * t).round().clamp(0.0, 255.0) as u32;

        (r << 16) | (g << 8) | b
    }

    pub fn default_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .map(|p| p.join("strata").join("theme.json"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            if let Some(cfg) = std::env::var_os("XDG_CONFIG_HOME") {
                Some(PathBuf::from(cfg).join("strata").join("theme.json"))
            } else {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|p| p.join(".config").join("strata").join("theme.json"))
            }
        }
    }

    pub fn load_persisted_or_default() -> Self {
        if let Some(path) = Self::default_config_path() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<CustomThemeConfig>(&contents) {
                    if let Ok(palette) = Self::from_custom_config(&cfg) {
                        return palette;
                    }
                }
            }
        }
        Self::aerostat_dark()
    }

    pub fn save_to_disk(&self) -> Result<PathBuf, String> {
        let path = Self::default_config_path()
            .ok_or_else(|| "Unable to resolve user config directory".to_string())?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&self.to_custom_config())
            .map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(path)
    }

    // Convenience GPUI Rgba accessors
    pub fn substrate_rgb(&self) -> Rgba {
        rgb(self.substrate)
    }
    pub fn primary_rgb(&self) -> Rgba {
        rgb(self.primary)
    }
    pub fn accent_rgb(&self) -> Rgba {
        rgb(self.accent)
    }
    pub fn telemetry_rgb(&self) -> Rgba {
        rgb(self.telemetry)
    }
    pub fn surface_lowest_rgb(&self) -> Rgba {
        rgb(self.surface_lowest)
    }
    pub fn surface_rgb(&self) -> Rgba {
        rgb(self.surface)
    }
    pub fn surface_container_rgb(&self) -> Rgba {
        rgb(self.surface_container)
    }
    pub fn surface_container_high_rgb(&self) -> Rgba {
        rgb(self.surface_container_high)
    }
    pub fn outline_rgb(&self) -> Rgba {
        rgb(self.outline)
    }
    pub fn outline_variant_rgb(&self) -> Rgba {
        rgb(self.outline_variant)
    }
    pub fn primary_container_rgb(&self) -> Rgba {
        rgb(self.primary_container)
    }
    pub fn primary_hover_rgb(&self) -> Rgba {
        rgb(self.primary_hover)
    }
    pub fn on_primary_rgb(&self) -> Rgba {
        rgb(self.on_primary)
    }
    pub fn accent_container_rgb(&self) -> Rgba {
        rgb(self.accent_container)
    }
    pub fn accent_hover_rgb(&self) -> Rgba {
        rgb(self.accent_hover)
    }
    pub fn on_accent_rgb(&self) -> Rgba {
        rgb(self.on_accent)
    }
    pub fn telemetry_container_rgb(&self) -> Rgba {
        rgb(self.telemetry_container)
    }
    pub fn text_primary_rgb(&self) -> Rgba {
        rgb(self.text_primary)
    }
    pub fn text_secondary_rgb(&self) -> Rgba {
        rgb(self.text_secondary)
    }
    pub fn text_muted_rgb(&self) -> Rgba {
        rgb(self.text_muted)
    }
    pub fn danger_rgb(&self) -> Rgba {
        rgb(self.danger)
    }
    pub fn danger_container_rgb(&self) -> Rgba {
        rgb(self.danger_container)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_aerostat_palettes() {
        let dark = ThemePalette::aerostat_dark();
        assert!(dark.is_dark);
        assert_eq!(dark.substrate, 0x1A2229);
        assert_eq!(dark.primary, 0x26466D);
        assert_eq!(dark.accent, 0xE08A63);
        assert_eq!(dark.telemetry, 0x6B9DA8);

        let light = ThemePalette::aerostat_light();
        assert!(!light.is_dark);
        assert_eq!(light.substrate, 0xD4DFE6);
        assert_eq!(light.primary, 0x26466D);
        assert_eq!(light.accent, 0xE08A63);
        assert_eq!(light.telemetry, 0x6B9DA8);
    }

    #[test]
    fn test_custom_theme_hex_roundtrip() {
        let mut theme = ThemePalette::aerostat_dark();
        theme
            .set_field_hex(ThemeTokenField::Substrate, "#D4DFE6")
            .unwrap();
        assert!(!theme.is_dark);
        assert_eq!(theme.substrate, 0xD4DFE6);

        let cfg = theme.to_custom_config();
        assert_eq!(cfg.substrate, "#D4DFE6");
        assert_eq!(cfg.primary, "#26466D");
        assert_eq!(cfg.accent, "#E08A63");
        assert_eq!(cfg.telemetry, "#6B9DA8");

        let restored = ThemePalette::from_custom_config(&cfg).unwrap();
        assert_eq!(restored.substrate, 0xD4DFE6);
        assert_eq!(restored.primary, 0x26466D);
    }
}

