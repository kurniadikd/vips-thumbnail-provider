use libloading::{Library, Symbol};
use std::ffi::{c_void, CString};
use std::sync::OnceLock;
use std::time::Instant;
use std::path::Path;

static LIBVIPS: OnceLock<Library> = OnceLock::new();
static LIBGOBJECT: OnceLock<Library> = OnceLock::new();

fn get_vips() -> &'static Library {
    LIBVIPS.get_or_init(|| unsafe {
        Library::new("libvips-42.dll").expect("Gagal memuat libvips-42.dll")
    })
}

fn get_gobject() -> &'static Library {
    LIBGOBJECT.get_or_init(|| unsafe {
        Library::new("libgobject-2.0-0.dll").expect("Gagal memuat libgobject-2.0-0.dll")
    })
}

type VipsInitFn = unsafe extern "C" fn(*const i8) -> i32;
type VipsThumbnailFn = unsafe extern "C" fn(*const i8, *mut *mut c_void, i32, ...) -> i32;
type GObjectUnrefFn = unsafe extern "C" fn(*mut c_void);

fn main() {
    let vips = get_vips();
    let gobject = get_gobject();

    let v_init: Symbol<VipsInitFn> = unsafe { vips.get(b"vips_init").expect("Symbol vips_init not found") };
    let v_thumb: Symbol<VipsThumbnailFn> = unsafe { vips.get(b"vips_thumbnail").expect("Symbol vips_thumbnail not found") };
    let g_unref: Symbol<GObjectUnrefFn> = unsafe { gobject.get(b"g_object_unref").expect("Symbol g_object_unref not found") };

    unsafe { v_init(CString::new("Benchmark").unwrap().as_ptr()); }

    let test_file = "input_image.png"; 
    if !Path::new(test_file).exists() {
        println!("File {} tidak ditemukan.", test_file);
        return;
    }

    println!("Memulai benchmark libvips untuk file: {}", test_file);
    
    let iterations = 20;
    let start = Instant::now();

    for _ in 0..iterations {
        unsafe {
            let mut img: *mut c_void = std::ptr::null_mut();
            let c_path = CString::new(test_file).unwrap();
            
            if v_thumb(c_path.as_ptr(), &mut img, 500, std::ptr::null_mut::<c_void>()) == 0 {
                g_unref(img);
            }
        }
    }

    let duration = start.elapsed();
    println!("--- HASIL BENCHMARK ---");
    println!("Total waktu ({}x proses): {:?}", iterations, duration);
    println!("Rata-rata per gambar: {:?}", duration / iterations);
    println!("-----------------------");
}
