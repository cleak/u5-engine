//! Chapter 9's colour transforms (Phase B design 9.3.2).
//!
//! The brightness scale works on light, not on colour numbers: a channel
//! is taken from sRGB to linear light, multiplied by the scale and taken
//! back. The functions are the usual sRGB transfer pair, with the
//! threshold value itself on the curved branch. The coverage curve is the
//! maps' tile-colour step (9.3.4); its user, the automap raster, comes in
//! a later milestone.
//!
//! Everything here is a pure CPU function over `f32` channels in 0..1 or
//! over `u8` bytes. There is no Bevy, settings, OS or GPU code (foundation
//! decision 5).

/// The coverage curve's exponent (9.3.2).
pub const BRIGHTNESS_CURVE_EXPONENT: f32 = 1.1;

/// sRGB-encoded channel (0..1) to linear light (0..1).
pub fn srgb_to_linear(c: f32) -> f32 {
    if c < 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear light to an sRGB-encoded channel, clamped to 0..1.
pub fn linear_to_srgb(l: f32) -> f32 {
    let l = l.max(0.0);
    let v = if l < 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    v.clamp(0.0, 1.0)
}

/// The maps' coverage curve: `coverage ^ 1.1`, times the user's scale
/// (9.3.4). `coverage` is clamped to 0..1 first.
pub fn coverage_scale(coverage: f32, user_scale: f32) -> f32 {
    coverage.clamp(0.0, 1.0).powf(BRIGHTNESS_CURVE_EXPONENT) * user_scale
}

/// One channel byte through the linear-light scale:
/// `linear_to_srgb(srgb_to_linear(in) * scale)`, rounded to a byte.
pub fn scale_channel(channel: u8, scale: f32) -> u8 {
    let lit = srgb_to_linear(f32::from(channel) / 255.0) * scale;
    // `linear_to_srgb` clamps to 0..1, so the cast cannot wrap.
    (linear_to_srgb(lit) * 255.0).round() as u8
}

/// The brightness table: every channel byte through [`scale_channel`] for
/// one scale, built once and read three times per pixel (9.3.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrightnessLut([u8; 256]);

impl BrightnessLut {
    /// The table for `scale` (the setting's percent divided by 100).
    pub fn new(scale: f32) -> Self {
        let mut table = [0u8; 256];
        for (value, out) in (0..=u8::MAX).zip(table.iter_mut()) {
            *out = scale_channel(value, scale);
        }
        Self(table)
    }

    /// The table that changes nothing. [`BrightnessLut::new`] at 1.0 is
    /// equal to it (a test).
    pub fn identity() -> Self {
        let mut table = [0u8; 256];
        for (value, out) in (0..=u8::MAX).zip(table.iter_mut()) {
            *out = value;
        }
        Self(table)
    }

    /// The table for a `display.brightness` percent.
    pub fn for_percent(percent: u8) -> Self {
        Self::new(f32::from(percent) / 100.0)
    }

    /// One channel byte through the table.
    #[inline]
    pub fn apply(&self, channel: u8) -> u8 {
        self.0[usize::from(channel)]
    }

    /// The R, G and B of each RGBA pixel through the table; alpha is kept.
    pub fn apply_rgba(&self, rgba: &mut [u8]) {
        for pixel in rgba.as_chunks_mut::<4>().0 {
            pixel[0] = self.apply(pixel[0]);
            pixel[1] = self.apply(pixel[1]);
            pixel[2] = self.apply(pixel[2]);
        }
    }

    /// The whole table, by input byte.
    pub fn table(&self) -> &[u8; 256] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_byte_survives_the_round_trip_within_one() {
        for value in 0..=u8::MAX {
            let back = (linear_to_srgb(srgb_to_linear(f32::from(value) / 255.0)) * 255.0).round();
            assert!(
                (back - f32::from(value)).abs() <= 1.0,
                "{value} came back as {back}"
            );
        }
    }

    #[test]
    fn the_transfer_pair_is_monotonic_and_pinned_at_the_ends() {
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-6);
        assert_eq!(linear_to_srgb(0.0), 0.0);
        assert!((linear_to_srgb(1.0) - 1.0).abs() < 1e-6);
        // Out-of-range light clamps instead of wrapping.
        assert_eq!(linear_to_srgb(-0.5), 0.0);
        assert_eq!(linear_to_srgb(4.0), 1.0);
        let mut last = -1.0f32;
        for value in 0..=u8::MAX {
            let l = srgb_to_linear(f32::from(value) / 255.0);
            assert!(l > last, "{value}");
            last = l;
        }
    }

    #[test]
    fn the_threshold_value_takes_the_curved_branch() {
        let curved = ((0.04045f32 + 0.055) / 1.055).powf(2.4);
        assert_eq!(srgb_to_linear(0.04045), curved);
        let curved = 1.055 * 0.003_130_8f32.powf(1.0 / 2.4) - 0.055;
        assert_eq!(linear_to_srgb(0.003_130_8), curved);
    }

    #[test]
    fn the_lut_at_full_scale_is_the_identity() {
        assert_eq!(BrightnessLut::new(1.0), BrightnessLut::identity());
        assert_eq!(BrightnessLut::for_percent(100), BrightnessLut::identity());
        for value in 0..=u8::MAX {
            assert_eq!(BrightnessLut::identity().apply(value), value);
        }
    }

    #[test]
    fn every_lut_is_monotonic_and_keeps_black() {
        for percent in 25..=100u8 {
            let lut = BrightnessLut::for_percent(percent);
            assert_eq!(lut.apply(0), 0, "{percent}%");
            for value in 1..=u8::MAX {
                assert!(
                    lut.apply(value) >= lut.apply(value - 1),
                    "{percent}% at {value}"
                );
                assert!(lut.apply(value) <= value, "{percent}% at {value}");
            }
        }
    }

    /// Design 9.2.2's table: the three EGA grey levels at each named stop.
    #[test]
    fn the_design_table_holds_at_every_named_stop() {
        let table: [(u8, [u8; 3]); 6] = [
            (100, [85, 170, 255]),
            (85, [79, 158, 237]),
            (70, [71, 145, 218]),
            (55, [63, 129, 196]),
            (40, [54, 112, 170]),
            (25, [42, 89, 137]),
        ];
        for (percent, expected) in table {
            let lut = BrightnessLut::for_percent(percent);
            let got = [lut.apply(0x55), lut.apply(0xAA), lut.apply(0xFF)];
            assert_eq!(got, expected, "{percent}%");
            assert_eq!(lut.apply(0), 0, "{percent}%");
        }
    }

    /// 9.2.2: the darkest EGA colours stay distinct from black down to the
    /// lowest setting.
    #[test]
    fn the_darkest_ega_level_stays_off_black_at_the_floor() {
        let lut = BrightnessLut::for_percent(25);
        assert!(lut.apply(0x55) > 0);
    }

    #[test]
    fn apply_rgba_keeps_alpha() {
        let lut = BrightnessLut::for_percent(70);
        let mut pixels = [0xAA, 0x55, 0xFF, 0x80, 0, 0, 0, 0xFF];
        lut.apply_rgba(&mut pixels);
        assert_eq!(pixels, [145, 71, 218, 0x80, 0, 0, 0, 0xFF]);
    }

    #[test]
    fn the_coverage_curve_is_the_power_times_the_scale() {
        assert_eq!(coverage_scale(0.0, 1.0), 0.0);
        assert!((coverage_scale(1.0, 1.0) - 1.0).abs() < 1e-6);
        assert!((coverage_scale(1.0, 0.7) - 0.7).abs() < 1e-6);
        assert!((coverage_scale(0.5, 1.0) - 0.5f32.powf(1.1)).abs() < 1e-6);
        // Clamped input.
        assert_eq!(coverage_scale(-1.0, 1.0), 0.0);
        assert!((coverage_scale(2.0, 0.5) - 0.5).abs() < 1e-6);
        let mut last = -1.0f32;
        for step in 0..=100 {
            let c = coverage_scale(step as f32 / 100.0, 1.0);
            assert!(c >= last);
            last = c;
        }
    }
}
