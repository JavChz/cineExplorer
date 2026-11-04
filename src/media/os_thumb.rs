#![allow(clippy::chunks_exact_to_as_chunks)]

use std::path::Path;

#[cfg(windows)]
pub fn extract_os_thumbnail<P: AsRef<Path>>(file_path: P, target_width: u32, target_height: u32) -> Result<Vec<u8>, String> {
    let (w, h, rgba) = extract_os_frame_rgba(file_path, target_width, target_height)?;
    let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|c| [c[0], c[1], c[2]]).collect();
    let mut jpeg_bytes = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 85);
    encoder.encode(&rgb, w, h, image::ExtendedColorType::Rgb8)
        .map_err(|e| format!("Failed to encode JPEG thumbnail: {}", e))?;
    Ok(jpeg_bytes)
}

#[cfg(windows)]
pub fn extract_os_frame_rgba<P: AsRef<Path>>(
    file_path: P,
    target_width: u32,
    target_height: u32,
) -> Result<(u32, u32, Vec<u8>), String> {
    use std::os::windows::ffi::OsStrExt;

    let path = file_path.as_ref();
    if !path.exists() {
        return Err(format!("File does not exist: {:?}", path));
    }

    let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();

    #[repr(C)]
    struct Size {
        cx: i32,
        cy: i32,
    }

    #[repr(C)]
    struct BitmapInfoHeader {
        bi_size: u32,
        bi_width: i32,
        bi_height: i32,
        bi_planes: u16,
        bi_bit_count: u16,
        bi_compression: u32,
        bi_size_image: u32,
        bi_xpels_per_meter: i32,
        bi_ypels_per_meter: i32,
        bi_clr_used: u32,
        bi_clr_important: u32,
    }

    #[repr(C)]
    struct BitmapInfo {
        bmi_header: BitmapInfoHeader,
        bmi_colors: [u32; 1],
    }

    #[repr(C)]
    struct Bitmap {
        bm_type: i32,
        bm_width: i32,
        bm_height: i32,
        bm_width_bytes: i32,
        bm_planes: u16,
        bm_bits_pixel: u16,
        bm_bits: *mut std::ffi::c_void,
    }

    #[link(name = "ole32")]
    extern "system" {
        fn CoInitializeEx(pv_reserved: *mut std::ffi::c_void, dw_co_init: u32) -> i32;
    }

    #[link(name = "shell32")]
    extern "system" {
        fn SHCreateItemFromParsingName(
            psz_path: *const u16,
            pbc: *mut std::ffi::c_void,
            riid: *const [u8; 16],
            ppv: *mut *mut std::ffi::c_void,
        ) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateCompatibleDC(hdc: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn DeleteDC(hdc: *mut std::ffi::c_void) -> i32;
        fn DeleteObject(ho: *mut std::ffi::c_void) -> i32;
        fn GetObjectW(h: *mut std::ffi::c_void, c: i32, pv: *mut std::ffi::c_void) -> i32;
        fn GetDIBits(
            hdc: *mut std::ffi::c_void,
            hbm: *mut std::ffi::c_void,
            start: u32,
            c_lines: u32,
            lpv_bits: *mut std::ffi::c_void,
            lpbmi: *mut BitmapInfo,
            usage: u32,
        ) -> i32;
    }

    // IShellItemImageFactory GUID: bcc18b79-ba16-442f-80c4-8a59c30c463b
    const IID_ISHELL_ITEM_IMAGE_FACTORY: [u8; 16] = [
        0x79, 0x8b, 0xc1, 0xbc, 0x16, 0xba, 0x2f, 0x44, 0x80, 0xc4, 0x8a, 0x59, 0xc3, 0x0c, 0x46, 0x3b,
    ];

    unsafe {
        let _ = CoInitializeEx(std::ptr::null_mut(), 0x0 /* COINIT_MULTITHREADED */);

        let mut factory_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let hr = SHCreateItemFromParsingName(
            wide_path.as_ptr(),
            std::ptr::null_mut(),
            &IID_ISHELL_ITEM_IMAGE_FACTORY,
            &mut factory_ptr,
        );

        if hr != 0 || factory_ptr.is_null() {
            return Err(format!("SHCreateItemFromParsingName failed with HRESULT 0x{:08X}", hr));
        }

        // COM vtable: QueryInterface(0), AddRef(1), Release(2), GetImage(3)
        let vtable = *(factory_ptr as *mut *mut usize);
        let get_image_fn: extern "system" fn(
            *mut std::ffi::c_void,
            Size,
            u32,
            *mut *mut std::ffi::c_void,
        ) -> i32 = std::mem::transmute(*vtable.add(3));
        let release_fn: extern "system" fn(*mut std::ffi::c_void) -> u32 =
            std::mem::transmute(*vtable.add(2));

        let mut hbitmap: *mut std::ffi::c_void = std::ptr::null_mut();
        let target_size = Size {
            cx: target_width as i32,
            cy: target_height as i32,
        };

        // Flags: 0x0 (default resize to fit)
        let img_hr = get_image_fn(factory_ptr, target_size, 0x0, &mut hbitmap);
        release_fn(factory_ptr);

        if img_hr != 0 || hbitmap.is_null() {
            return Err(format!("IShellItemImageFactory::GetImage failed with HRESULT 0x{:08X}", img_hr));
        }

        // Inspect HBITMAP dimensions
        let mut bm: Bitmap = std::mem::zeroed();
        let get_obj_res = GetObjectW(
            hbitmap,
            std::mem::size_of::<Bitmap>() as i32,
            &mut bm as *mut _ as *mut std::ffi::c_void,
        );

        if get_obj_res == 0 || bm.bm_width <= 0 || bm.bm_height <= 0 {
            DeleteObject(hbitmap);
            return Err("GetObjectW failed on extracted HBITMAP".to_string());
        }

        let actual_width = bm.bm_width as u32;
        let actual_height = bm.bm_height.unsigned_abs();

        let hdc = CreateCompatibleDC(std::ptr::null_mut());
        if hdc.is_null() {
            DeleteObject(hbitmap);
            return Err("CreateCompatibleDC failed".to_string());
        }

        let mut bmi: BitmapInfo = std::mem::zeroed();
        bmi.bmi_header.bi_size = std::mem::size_of::<BitmapInfoHeader>() as u32;
        bmi.bmi_header.bi_width = actual_width as i32;
        // Negative bi_height specifies top-down DIB rows
        bmi.bmi_header.bi_height = -(actual_height as i32);
        bmi.bmi_header.bi_planes = 1;
        bmi.bmi_header.bi_bit_count = 32;
        bmi.bmi_header.bi_compression = 0; // BI_RGB

        let pixel_count = (actual_width * actual_height) as usize;
        let mut bgra_buf: Vec<u8> = vec![0u8; pixel_count * 4];

        let lines_copied = GetDIBits(
            hdc,
            hbitmap,
            0,
            actual_height,
            bgra_buf.as_mut_ptr() as *mut std::ffi::c_void,
            &mut bmi,
            0, // DIB_RGB_COLORS
        );

        DeleteDC(hdc);
        DeleteObject(hbitmap);

        if lines_copied <= 0 {
            return Err("GetDIBits failed to copy pixel data".to_string());
        }

        // Convert BGRA to RGBA in place
        for chunk in bgra_buf.chunks_exact_mut(4) {
            let b = chunk[0];
            let r = chunk[2];
            chunk[0] = r;
            chunk[2] = b;
            // Ensure full alpha if shell returned 0 alpha
            if chunk[3] == 0 {
                chunk[3] = 255;
            }
        }

        Ok((actual_width, actual_height, bgra_buf))
    }
}

#[cfg(not(windows))]
pub fn extract_os_thumbnail<P: AsRef<Path>>(_file_path: P, _target_width: u32, _target_height: u32) -> Result<Vec<u8>, String> {
    Err("OS thumbnail extraction is only supported on Windows".to_string())
}

#[cfg(not(windows))]
pub fn extract_os_frame_rgba<P: AsRef<Path>>(
    _file_path: P,
    _target_width: u32,
    _target_height: u32,
) -> Result<(u32, u32, Vec<u8>), String> {
    Err("OS frame extraction is only supported on Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_os_thumbnail_braw() {
        let braw = Path::new("P:\\VideoRepo\\BMPCC\\2026-01-21\\A009_03112214_C001.braw");
        if braw.exists() {
            let res = extract_os_thumbnail(braw, 480, 270);
            assert!(res.is_ok(), "OS thumbnail extraction should succeed for BRAW: {:?}", res.err());
            let bytes = res.unwrap();
            assert!(!bytes.is_empty());
            assert_eq!(&bytes[0..2], &[0xFF, 0xD8]); // Valid JPEG SOI
        }
    }
}
