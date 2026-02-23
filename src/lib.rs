use libloading::{Library, Symbol};
use std::ffi::{c_void, CString};
use std::sync::OnceLock;
use windows::{
    core::{implement, Result, HRESULT, Error, GUID, IUnknown, Interface},
    Win32::Foundation::BOOL,
    Win32::Graphics::Gdi::{CreateBitmap, HBITMAP},
    Win32::System::Com::{IClassFactory, IClassFactory_Impl},
    Win32::System::SystemServices::DLL_PROCESS_ATTACH,
    Win32::UI::Shell::{IThumbnailProvider, IThumbnailProvider_Impl, WTS_ALPHATYPE, WTSAT_ARGB},
    Win32::UI::Shell::PropertiesSystem::{IInitializeWithFile, IInitializeWithFile_Impl},
    Win32::System::Diagnostics::Debug::OutputDebugStringW,
    Win32::System::LibraryLoader::SetDllDirectoryW,
};
use std::sync::Mutex;

static SYMBOLS: OnceLock<VipsSymbols> = OnceLock::new();

struct VipsSymbols {
    v_init: Symbol<'static, VipsInitFn>,
    v_concurrency: Symbol<'static, VipsConcurrencySetFn>,
    v_thumb: Symbol<'static, VipsThumbnailFn>,
    v_colour: Symbol<'static, VipsColourspaceFn>,
    v_alpha: Symbol<'static, VipsAddAlphaFn>,
    v_data: Symbol<'static, VipsImageGetDataFn>,
    v_w: Symbol<'static, VipsImageGetWidthFn>,
    v_h: Symbol<'static, VipsImageGetHeightFn>,
    v_bands: Symbol<'static, VipsImageGetBandsFn>,
    v_interpretation: Symbol<'static, VipsImageGetInterpretationFn>,
    g_unref: Symbol<'static, GObjectUnrefFn>,
}

fn get_symbols() -> Option<&'static VipsSymbols> {
    if SYMBOLS.get().is_none() {
        unsafe {
            let bin_path: Vec<u16> = "C:\\vips\\bin\0".encode_utf16().collect();
            let _ = SetDllDirectoryW(windows::core::PCWSTR(bin_path.as_ptr()));

            let lib_vips = Library::new("C:\\vips\\bin\\libvips-42.dll").ok()?;
            let lib_gobject = Library::new("C:\\vips\\bin\\libgobject-2.0-0.dll").ok()?;
            
            let lib_vips = Box::leak(Box::new(lib_vips));
            let lib_gobject = Box::leak(Box::new(lib_gobject));

            let sym = VipsSymbols {
                v_init: lib_vips.get(b"vips_init").ok()?,
                v_concurrency: lib_vips.get(b"vips_concurrency_set").ok()?,
                v_thumb: lib_vips.get(b"vips_thumbnail").ok()?,
                v_colour: lib_vips.get(b"vips_colourspace").ok()?,
                v_alpha: lib_vips.get(b"vips_addalpha").ok()?,
                v_data: lib_vips.get(b"vips_image_get_data").ok()?,
                v_w: lib_vips.get(b"vips_image_get_width").ok()?,
                v_h: lib_vips.get(b"vips_image_get_height").ok()?,
                v_bands: lib_vips.get(b"vips_image_get_bands").ok()?,
                v_interpretation: lib_vips.get(b"vips_image_get_interpretation").ok()?,
                g_unref: lib_gobject.get(b"g_object_unref").ok()?,
            };
            
            let app_name = CString::new("VipsThumbExt").unwrap();
            (sym.v_init)(app_name.as_ptr());
            (sym.v_concurrency)(4); // Keseimbangan yang baik antara speed dan resource

            let _ = SYMBOLS.set(sym);
        }
    }
    SYMBOLS.get()
}

fn log_msg(msg: &str) {
    let wide: Vec<u16> = format!("VipsThumb: {}\0", msg).encode_utf16().collect();
    unsafe {
        OutputDebugStringW(windows::core::PCWSTR(wide.as_ptr()));
    }
}

type VipsInitFn = unsafe extern "C" fn(*const i8) -> i32;
type VipsConcurrencySetFn = unsafe extern "C" fn(i32);
type VipsThumbnailFn = unsafe extern "C" fn(*const i8, *mut *mut c_void, i32, ...) -> i32;
type VipsColourspaceFn = unsafe extern "C" fn(*mut c_void, *mut *mut c_void, i32, ...) -> i32;
type VipsAddAlphaFn = unsafe extern "C" fn(*mut c_void, *mut *mut c_void, ...) -> i32;
type VipsImageGetDataFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type VipsImageGetWidthFn = unsafe extern "C" fn(*mut c_void) -> i32;
type VipsImageGetHeightFn = unsafe extern "C" fn(*mut c_void) -> i32;
type VipsImageGetBandsFn = unsafe extern "C" fn(*mut c_void) -> i32;
type VipsImageGetInterpretationFn = unsafe extern "C" fn(*mut c_void) -> i32;
type GObjectUnrefFn = unsafe extern "C" fn(*mut c_void);

#[implement(IThumbnailProvider, IInitializeWithFile)]
struct VipsThumbnailProvider_Impl {
    file_path: Mutex<String>,
}

impl IInitializeWithFile_Impl for VipsThumbnailProvider_Impl {
    fn Initialize(&self, pszfilepath: &windows::core::PCWSTR, _dwmode: u32) -> Result<()> {
        let mut path = self.file_path.lock().unwrap();
        unsafe {
            *path = pszfilepath.to_string()?;
        }
        Ok(())
    }
}

impl IThumbnailProvider_Impl for VipsThumbnailProvider_Impl {
    fn GetThumbnail(&self, cx: u32, phbmp: *mut HBITMAP, pdwalpha: *mut WTS_ALPHATYPE) -> Result<()> {
        let path = self.file_path.lock().unwrap().clone();
        if path.is_empty() { return Err(Error::from_hresult(HRESULT(-1))); }

        let sym = get_symbols().ok_or_else(|| Error::from_hresult(HRESULT(-1)))?;

        unsafe {
            let start = std::time::Instant::now();
            let target_cx = cx.min(96);
            
            // 1. Generate Thumbnail - Menggunakan hint untuk AVIF/HEIC
            let mut img: *mut c_void = std::ptr::null_mut();
            // Hint [thumbnail=true,n=1,access=sequential] untuk kecepatan maksimal
            let path_with_hints = format!("{}[thumbnail=true,n=1,access=sequential]", path.replace("\\", "/"));
            let c_path = CString::new(path_with_hints).unwrap();
            
            let height_key = CString::new("height").unwrap();
            let size_key = CString::new("size").unwrap();
            let auto_rotate_key = CString::new("auto_rotate").unwrap();
            let kernel_key = CString::new("kernel").unwrap();
            
            if (sym.v_thumb)(
                c_path.as_ptr(),
                &mut img, 
                target_cx as i32, 
                height_key.as_ptr(), target_cx as i32,
                size_key.as_ptr(), 1, // VIPS_SIZE_DOWN
                kernel_key.as_ptr(), 0, // VIPS_KERNEL_NEAREST (Paling cepat, kualitas terendah)
                auto_rotate_key.as_ptr(), 1,
                std::ptr::null::<c_void>()
            ) != 0 {
                return Err(Error::from_hresult(HRESULT(-1)));
            }

            // 2. Convert to sRGB only if NOT already sRGB
            let mut final_img_tmp: *mut c_void = std::ptr::null_mut();
            let current_interp = (sym.v_interpretation)(img);
            if current_interp != 22 /* sRGB */ {
                if (sym.v_colour)(img, &mut final_img_tmp, 22 /* sRGB */, std::ptr::null_mut::<c_void>()) != 0 {
                    (sym.g_unref)(img);
                    return Err(Error::from_hresult(HRESULT(-1)));
                }
                (sym.g_unref)(img);
            } else {
                final_img_tmp = img;
            }
            let srgb_img = final_img_tmp;

            // 3. Ensure 32-bit (BGRA) for Windows GDI
            let final_img: *mut c_void;
            let mut alpha_img: *mut c_void = std::ptr::null_mut();
            if (sym.v_bands)(srgb_img) != 4 {
                if (sym.v_alpha)(srgb_img, &mut alpha_img, std::ptr::null_mut::<c_void>()) != 0 {
                    (sym.g_unref)(srgb_img);
                    return Err(Error::from_hresult(HRESULT(-1)));
                }
                (sym.g_unref)(srgb_img);
                final_img = alpha_img;
            } else {
                final_img = srgb_img;
            }

            // 4. Create HBITMAP and pass to Windows
            let width = (sym.v_w)(final_img) as i32;
            let height = (sym.v_h)(final_img) as i32;
            let memory_buffer = (sym.v_data)(final_img);

            let hbitmap = CreateBitmap(width, height, 1, 32, Some(memory_buffer as *const _));
            (sym.g_unref)(final_img);

            if hbitmap.is_invalid() {
                return Err(Error::from_hresult(HRESULT(-1)));
            }
            
            *phbmp = hbitmap;
            *pdwalpha = WTSAT_ARGB;
        }

        Ok(())
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    HRESULT(0) // S_OK
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllGetClassObject(rclsid: *const GUID, riid: *const GUID, ppv: *mut *mut c_void) -> HRESULT {
    unsafe {
        if *rclsid == GUID::from_u128(0xD3A2F1B2_7E8B_4C9D_A3D1_2F0B3C4D5E6F) {
            let factory: IClassFactory = VipsThumbnailProviderFactory_Impl {}.into();
            Interface::query(&factory, riid, ppv)
        } else {
            HRESULT(-2147221231) // CLASS_E_CLASSNOTAVAILABLE
        }
    }
}

#[implement(IClassFactory)]
struct VipsThumbnailProviderFactory_Impl {}

impl IClassFactory_Impl for VipsThumbnailProviderFactory_Impl {
    fn CreateInstance(&self, _punkouter: Option<&IUnknown>, riid: *const GUID, ppv: *mut *mut c_void) -> Result<()> {
        unsafe {
            let provider: IThumbnailProvider = VipsThumbnailProvider_Impl {
                file_path: Mutex::new(String::new()),
            }.into();
            Interface::query(&provider, riid, ppv).ok()
        }
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(_hmodule: *mut c_void, dwreason: u32, _lpreserved: *mut c_void) -> bool {
    if dwreason == DLL_PROCESS_ATTACH {
        // Initialization handled in OnceLock
    }
    true
}
