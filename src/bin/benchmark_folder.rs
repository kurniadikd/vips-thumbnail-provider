use libloading::{Library, Symbol};
use windows::{
    core::{GUID, Result},
    Win32::UI::Shell::IThumbnailProvider,
    Win32::Graphics::Gdi::HBITMAP,
};
use std::ffi::c_void;
use std::time::Instant;

type DllGetClassObjectFn = unsafe extern "system" fn(rclsid: *const GUID, riid: *const GUID, ppv: *mut *mut c_void) -> windows::core::HRESULT;
type VipsImageGetWidthFn = unsafe extern "C" fn(*mut c_void) -> i32;
type VipsImageGetHeightFn = unsafe extern "C" fn(*mut c_void) -> i32;
type VipsImageGetBandsFn = unsafe extern "C" fn(*mut c_void) -> i32;
type VipsImageNewFromFileFn = unsafe extern "C" fn(*const i8, ...) -> *mut c_void;
type GObjectUnrefFn = unsafe extern "C" fn(*mut c_void);

fn main() -> Result<()> {
    let folder = r#"C:\Folder_D\backup\310125-"#;
    let paths: Vec<_> = std::fs::read_dir(folder)
        .expect("Gagal membaca folder")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "avif"))
        .take(10) // Ambil 10 sampel
        .collect();

    if paths.is_empty() {
        println!("Tidak ditemukan file AVIF di folder tersebut.");
        return Ok(());
    }

    unsafe {
        let vips_bin = Library::new("C:\\vips\\bin\\libvips-42.dll").expect("Gagal memuat libvips");
        let v_new_from_file: Symbol<VipsImageNewFromFileFn> = vips_bin.get(b"vips_image_new_from_file").expect("v_new_from_file");
        let v_w: Symbol<VipsImageGetWidthFn> = vips_bin.get(b"vips_image_get_width").expect("v_w");
        let v_h: Symbol<VipsImageGetHeightFn> = vips_bin.get(b"vips_image_get_height").expect("v_h");
        let v_bands: Symbol<VipsImageGetBandsFn> = vips_bin.get(b"vips_image_get_bands").expect("v_bands");
        let lib_gobject = Library::new("C:\\vips\\bin\\libgobject-2.0-0.dll").expect("G");
        let g_unref: Symbol<GObjectUnrefFn> = lib_gobject.get(b"g_object_unref").expect("u");
        let v_init: Symbol<unsafe extern "C" fn(*const i8) -> i32> = vips_bin.get(b"vips_init").expect("v_init");
        v_init(std::ptr::null());

        let lib = Library::new("target\\release\\thumbnail_generator.dll").expect("Gagal memuat DLL");
        let get_class_object: Symbol<DllGetClassObjectFn> = lib.get(b"DllGetClassObject").expect("Simbol DllGetClassObject tidak ditemukan");
        
        let clsid = GUID::from_u128(0xD3A2F1B2_7E8B_4C9D_A3D1_2F0B3C4D5E6F);
        let iid_class_factory = GUID::from_u128(0x00000001_0000_0000_C000_000000000046); 
        
        let mut factory_ptr: *mut c_void = std::ptr::null_mut();
        get_class_object(&clsid, &iid_class_factory, &mut factory_ptr);
        let factory: windows::Win32::System::Com::IClassFactory = std::mem::transmute(factory_ptr);
        
        let provider: IThumbnailProvider = factory.CreateInstance(None).expect("Gagal membuat instance provider");
        // Kita perlu IInitializeWithFile juga
        let initializer: windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile = windows::core::Interface::cast(&provider).expect("Gagal cast ke IInitializeWithFile");

        println!("Memulai Benchmark untuk {} file AVIF...\n", paths.len());

        let total_start = Instant::now();
        for path in &paths {
            let path_str = path.to_str().unwrap();
            let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
            let pcwstr = windows::core::PCWSTR(wide_path.as_ptr());

            let init_start = Instant::now();
            initializer.Initialize(pcwstr, 0).expect("Gagal inisialisasi file");
            
            let mut hbmp: HBITMAP = HBITMAP::default();
            let mut alpha: windows::Win32::UI::Shell::WTS_ALPHATYPE = windows::Win32::UI::Shell::WTSAT_UNKNOWN;
            
            let thumb_start = Instant::now();
            let _ = provider.GetThumbnail(256, &mut hbmp, &mut alpha);
            let elapsed = thumb_start.elapsed();
            
            println!("File: {} -> Total: {:?}", 
                path.file_name().unwrap().to_str().unwrap(), 
                elapsed
            );
        }
        
        println!("\nRata-rata waktu per file: {:?}", total_start.elapsed() / paths.len() as u32);
    }

    Ok(())
}
