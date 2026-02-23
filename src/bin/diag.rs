use libloading::{Library, Symbol};
use windows::{
    core::{GUID, Result},
    Win32::UI::Shell::IThumbnailProvider,
    Win32::Graphics::Gdi::HBITMAP,
};
use std::ffi::c_void;

type DllGetClassObjectFn = unsafe extern "system" fn(rclsid: *const GUID, riid: *const GUID, ppv: *mut *mut c_void) -> windows::core::HRESULT;

fn main() -> Result<()> {
    unsafe {
        println!("Memuat DLL...");
        let lib = Library::new("target\\release\\thumbnail_generator.dll").expect("Gagal memuat DLL");
        
        let get_class_object: Symbol<DllGetClassObjectFn> = lib.get(b"DllGetClassObject").expect("Simbol DllGetClassObject tidak ditemukan");
        
        let clsid = GUID::from_u128(0xD3A2F1B2_7E8B_4C9D_A3D1_2F0B3C4D5E6F);
        let iid_class_factory = GUID::from_u128(0x00000001_0000_0000_C000_000000000046); // IClassFactory
        
        let mut factory: *mut c_void = std::ptr::null_mut();
        let hr = get_class_object(&clsid, &iid_class_factory, &mut factory);
        
        if hr.is_err() {
            println!("DllGetClassObject gagal: {:?}", hr);
            return Err(windows::core::Error::from_hresult(hr));
        }
        println!("Class Factory berhasil didapatkan!");
        let factory: windows::Win32::System::Com::IClassFactory = std::mem::transmute(factory);
        
        let provider: IThumbnailProvider = factory.CreateInstance(None).expect("Gagal membuat instance provider");
        println!("Provider instance berhasil dibuat!");

        // Kita panggil Initialize lewat interface IInitializeWithFile (jika diimplementasi)
        // Namun untuk tes log_msg, kita cukup panggil GetThumbnail yang akan gagal karena path kosong
        // tapi seharusnya men-trigger log_msg.
        println!("Memanggil GetThumbnail 10x untuk benchmark...");
        for i in 1..=10 {
            let start = std::time::Instant::now();
            let mut hbmp: HBITMAP = HBITMAP::default();
            let mut alpha: windows::Win32::UI::Shell::WTS_ALPHATYPE = windows::Win32::UI::Shell::WTSAT_UNKNOWN;
            
            // Kita panggil dengan path kosong dulu untuk tes inisialisasi awal
            let _ = provider.GetThumbnail(256, &mut hbmp, &mut alpha);
            println!("Iterasi {}: {:?}", i, start.elapsed());
        }

        println!("Selesai. Cek OutputDebugString (DebugView) untuk melihat log dari DLL.");
        
        // Cek apakah log ada
        if std::path::Path::new("C:\\vips\\vips_thumb.log").exists() {
            println!("LOG DITEMUKAN!");
        } else {
            println!("LOG TIDAK DITEMUKAN. Ada masalah di fungsi log_msg.");
        }
    }
    Ok(())
}
