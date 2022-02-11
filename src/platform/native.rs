use bevy::utils::BoxedFuture;
use std::{fs::File, io::Read};

#[derive(Default, Clone, Copy)]
pub struct FileApiImpl;

#[derive(Debug)]
pub enum FileError {
    Error,
}

impl super::FileApi for FileApiImpl {
    type FileError = std::io::Error;
    fn read<'a>(&'a self, path: &'a str) -> BoxedFuture<'a, Result<Vec<u8>, Self::FileError>> {
        bevy::utils::tracing::info!("reading {}", path);
        Box::pin(async move {
            let mut file = File::open(path)?;
            let mut data = vec![];
            file.read_to_end(&mut data);
            // js_api::readFile(path)
            //     .await
            //     .map(|result| js_sys::Uint8Array::from(result).to_vec())
            //     .map_err(|e| JsFileError::Error(e))
            Ok(data)
        })
    }

    fn write<'a>(
        &'a self,
        path: &'a str,
        contents: &'a [u8],
    ) -> BoxedFuture<'a, Result<(), Self::FileError>> {
        Box::pin(async move {
            // js_api::writeFile(path, contents)
            //     .await
            //     .map_err(|e| JsFileError::Error(e))
            Ok(())
        })
    }

    fn read_dir<'a>(
        &'a self,
        path: &'a str,
    ) -> BoxedFuture<'a, Result<Vec<String>, Self::FileError>> {
        Box::pin(async move {
            // js_api::ls(path)
            //     .await
            //     .map(|result| {
            //         let mut files = Vec::new();
            //         // js_sys::Array::from(&result).iter().map(|v| v.as_string())
            //         for item in js_sys::Array::from(&result).iter() {
            //             files.push(item.as_string().unwrap());
            //         }
            //         files
            //     })
            //     .map_err(|e| JsFileError::Error(e))
            Ok(vec![])
        })
    }
}
