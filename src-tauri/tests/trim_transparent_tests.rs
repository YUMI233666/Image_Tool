use art_tool_lib::core::processors::trim_transparent::{
  calculate_non_transparent_bbox,
  BoundingBox,
};
use image::{Rgba, RgbaImage};

fn transparent_image(width: u32, height: u32) -> RgbaImage {
  RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]))
}

#[test]
fn returns_none_for_fully_transparent_image() {
  let image = transparent_image(8, 8);
  let bbox = calculate_non_transparent_bbox(&image, 0);
  assert!(bbox.is_none());
}

#[test]
fn calculates_bbox_for_centered_opaque_block() {
  let mut image = transparent_image(12, 12);

  for y in 3..=7 {
    for x in 2..=8 {
      image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
    }
  }

  let bbox = calculate_non_transparent_bbox(&image, 0);

  assert_eq!(
    bbox,
    Some(BoundingBox {
      left: 2,
      top: 3,
      right: 8,
      bottom: 7,
    })
  );

  let got = bbox.expect("bbox should exist");
  assert_eq!(got.width(), 7);
  assert_eq!(got.height(), 5);
}

#[test]
fn alpha_threshold_filters_weak_pixels() {
  let mut image = transparent_image(10, 10);
  image.put_pixel(1, 1, Rgba([255, 255, 255, 10]));
  image.put_pixel(6, 4, Rgba([255, 255, 255, 220]));

  let bbox = calculate_non_transparent_bbox(&image, 200);

  assert_eq!(
    bbox,
    Some(BoundingBox {
      left: 6,
      top: 4,
      right: 6,
      bottom: 4,
    })
  );
}
