use crate::js_api;
use wasm_bindgen::JsValue;
use bevy::utils::BoxedFuture;


#[derive(Default, Clone, Copy)]
pub struct FileApiImpl;

#[derive(Debug)]
pub enum JsFileError {
    Error(JsValue),
}

impl super::FileApi for FileApiImpl {
    type FileError = JsFileError;
    fn read<'a>(&'a self, path: &'a str) -> BoxedFuture<'a, Result<Vec<u8>, JsFileError>> {
        Box::pin(async move {
            js_api::readFile(path)
                .await
                .map(|result| js_sys::Uint8Array::from(result).to_vec())
                .map_err(|e| JsFileError::Error(e))
        })
    }

    fn write<'a>(
        &'a self,
        path: &'a str,
        contents: &'a [u8],
    ) -> BoxedFuture<'a, Result<(), JsFileError>> {
        Box::pin(async move {
            js_api::writeFile(path, contents)
                .await
                .map_err(|e| JsFileError::Error(e))
        })
    }

    fn read_dir<'a>(&'a self, path: &'a str) -> BoxedFuture<'a, Result<Vec<String>, JsFileError>> {
        Box::pin(async move {
            js_api::ls(path)
                .await
                .map(|result| {
                    let mut files = Vec::new();
                    // js_sys::Array::from(&result).iter().map(|v| v.as_string())
                    for item in js_sys::Array::from(&result).iter() {
                        files.push(item.as_string().unwrap());
                    }
                    files
                })
                .map_err(|e| JsFileError::Error(e))
        })
    }
}
