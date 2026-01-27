/// Box blur implementation with separable passes for efficient software rendering.
///
/// This module provides a fast box blur algorithm that processes images in two passes:
/// 1. Horizontal pass - blurs each row
/// 2. Vertical pass - blurs each column
///
/// This separable approach is O(n) per pixel instead of O(n²), making it very
/// performant for software rendering.

/// Apply box blur to RGBA image data.
///
/// # Arguments
/// * `data` - Mutable slice of RGBA pixel data (premultiplied alpha)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `radius` - Blur radius (higher = more blur)
///
/// # Note
/// The data is modified in-place. Format is expected to be RGBA with 4 bytes per pixel.
pub fn box_blur(data: &mut [u8], width: usize, height: usize, radius: u32) {
    if radius == 0 || width == 0 || height == 0 {
        return;
    }

    let radius = radius as usize;

    // Allocate temporary buffer for intermediate results
    let mut temp = vec![0u8; data.len()];

    // Horizontal pass
    box_blur_horizontal(data, &mut temp, width, height, radius);

    // Vertical pass (reads from temp, writes to data)
    box_blur_vertical(&temp, data, width, height, radius);
}

/// Horizontal blur pass - process each row independently
fn box_blur_horizontal(src: &[u8], dst: &mut [u8], width: usize, height: usize, radius: usize) {
    for y in 0..height {
        for x in 0..width {
            let mut r_sum = 0u32;
            let mut g_sum = 0u32;
            let mut b_sum = 0u32;
            let mut a_sum = 0u32;
            let mut count = 0u32;

            // Calculate bounds for the box
            let x_min = x.saturating_sub(radius);
            let x_max = (x + radius + 1).min(width);

            // Sum all pixels in the horizontal box
            for bx in x_min..x_max {
                let idx = (y * width + bx) * 4;
                r_sum += src[idx] as u32;
                g_sum += src[idx + 1] as u32;
                b_sum += src[idx + 2] as u32;
                a_sum += src[idx + 3] as u32;
                count += 1;
            }

            // Write averaged result
            let dst_idx = (y * width + x) * 4;
            dst[dst_idx] = (r_sum / count) as u8;
            dst[dst_idx + 1] = (g_sum / count) as u8;
            dst[dst_idx + 2] = (b_sum / count) as u8;
            dst[dst_idx + 3] = (a_sum / count) as u8;
        }
    }
}

/// Vertical blur pass - process each column independently
fn box_blur_vertical(src: &[u8], dst: &mut [u8], width: usize, height: usize, radius: usize) {
    for x in 0..width {
        for y in 0..height {
            let mut r_sum = 0u32;
            let mut g_sum = 0u32;
            let mut b_sum = 0u32;
            let mut a_sum = 0u32;
            let mut count = 0u32;

            // Calculate bounds for the box
            let y_min = y.saturating_sub(radius);
            let y_max = (y + radius + 1).min(height);

            // Sum all pixels in the vertical box
            for by in y_min..y_max {
                let idx = (by * width + x) * 4;
                r_sum += src[idx] as u32;
                g_sum += src[idx + 1] as u32;
                b_sum += src[idx + 2] as u32;
                a_sum += src[idx + 3] as u32;
                count += 1;
            }

            // Write averaged result
            let dst_idx = (y * width + x) * 4;
            dst[dst_idx] = (r_sum / count) as u8;
            dst[dst_idx + 1] = (g_sum / count) as u8;
            dst[dst_idx + 2] = (b_sum / count) as u8;
            dst[dst_idx + 3] = (a_sum / count) as u8;
        }
    }
}

/// Apply multiple passes of box blur to approximate Gaussian blur.
///
/// Multiple passes of box blur converge towards Gaussian blur quality.
/// 3 passes is usually a good balance between quality and performance.
///
/// # Arguments
/// * `data` - Mutable slice of RGBA pixel data
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `radius` - Blur radius per pass
/// * `passes` - Number of blur passes (typically 2-3)
pub fn box_blur_multi_pass(data: &mut [u8], width: usize, height: usize, radius: u32, passes: u32) {
    for _ in 0..passes {
        box_blur(data, width, height, radius);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_blur_empty() {
        let mut data = vec![];
        box_blur(&mut data, 0, 0, 5);
        // Should not panic
    }

    #[test]
    fn test_box_blur_zero_radius() {
        let mut data = vec![255u8; 16]; // 2x2 image
        let original = data.clone();
        box_blur(&mut data, 2, 2, 0);
        assert_eq!(data, original); // Should be unchanged
    }

    #[test]
    fn test_box_blur_single_pixel() {
        let mut data = vec![255, 128, 64, 255]; // 1x1 image
        let original = data.clone();
        box_blur(&mut data, 1, 1, 5);
        assert_eq!(data, original); // Single pixel can't be blurred
    }

    #[test]
    fn test_box_blur_small_image() {
        // 2x2 image with different colors
        let mut data = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 0, 255, // Yellow
        ];

        box_blur(&mut data, 2, 2, 1);

        // After blur, all pixels should be averaged
        // Each pixel should be influenced by its neighbors
        for i in (0..16).step_by(4) {
            let r = data[i];
            let g = data[i + 1];
            let b = data[i + 2];

            // Colors should be mixed, not pure anymore
            assert!(r > 0 && r < 255);
            assert!(g > 0 && g < 255);
            assert!(b > 0 && b < 200); // Blue might be lower
        }
    }
}
