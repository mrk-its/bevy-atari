use bevy::prelude::{EventWriter, Res};
use crossbeam_channel::{Receiver, Sender};
use std::future::Future;
use std::sync::Arc;

use bevy::tasks::TaskPool;
use bevy::utils::BoxedFuture;

#[cfg(target_arch = "wasm32")]
#[path = "web.rs"]
mod fs_impl;

#[cfg(not(target_arch = "wasm32"))]
#[path = "native.rs"]
mod fs_impl;

#[derive(Debug)]
pub enum FsEvent {
    AttachBinary {
        key: String,
        path: String,
        data: Vec<u8>,
    },
    FileList(String, Vec<String>),
    File(String, Vec<u8>),
    Written(String),
}

pub trait FileApi {
    type FileError: std::fmt::Debug;
    fn read<'a>(&'a self, path: &'a str) -> BoxedFuture<'a, Result<Vec<u8>, Self::FileError>>;
    fn write<'a>(
        &'a self,
        path: &'a str,
        content: &'a [u8],
    ) -> BoxedFuture<'a, Result<(), Self::FileError>>;
    fn read_dir<'a>(
        &'a self,
        path: &'a str,
    ) -> BoxedFuture<'a, Result<Vec<String>, Self::FileError>>;
}

pub struct FileSystemInternal {
    api: fs_impl::FileApiImpl,
    task_pool: TaskPool,
    pub sender: Sender<Option<FsEvent>>,
    pub receiver: Receiver<Option<FsEvent>>,
}

impl FileSystemInternal {
    pub fn read_dir(
        &self,
        path: &'static str,
        create_response: impl FnOnce(Vec<String>) -> FsEvent + Send + 'static,
    ) {
        let api = self.api.clone();
        self.file_op(async move { api.read_dir(path).await.map(create_response) });
    }

    pub fn read(
        &self,
        path: &'static str,
        create_response: impl FnOnce(Vec<u8>) -> FsEvent + Send + 'static,
    ) {
        let api = self.api.clone();
        self.file_op(async move { api.read(path).await.map(create_response) });
    }

    pub fn write(
        &self,
        path: &str,
        contents: Vec<u8>,
        create_response: impl FnOnce(()) -> FsEvent + Send + 'static,
    ) {
        let api = self.api.clone();
        let path = path.to_owned();
        self.file_op(async move { api.write(&path, &contents).await.map(create_response) });
    }

    pub fn file_op(
        &self,
        #[cfg(target_arch = "wasm32")] future: impl Future<Output = Result<FsEvent, <fs_impl::FileApiImpl as FileApi>::FileError>>
            + 'static,
        #[cfg(not(target_arch = "wasm32"))] future: impl Future<Output = Result<FsEvent, <fs_impl::FileApiImpl as FileApi>::FileError>>
            + 'static
            + Send,
    ) {
        let sender = self.sender.clone();
        self.task_pool
            .spawn(async move {
                let x = future.await;
                let response = match x {
                    Ok(response) => sender.send(Some(response)),
                    _ => {
                        bevy::log::warn!("{:?}", x);
                        return;
                    }
                };
                bevy::log::info!("async fs response: {:?}", response);
            })
            .detach();
    }
}

pub struct FileSystem {
    pub(crate) inner: Arc<FileSystemInternal>,
}

impl FileSystem {
    pub fn new(task_pool: TaskPool) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self {
            inner: Arc::new(FileSystemInternal {
                api: fs_impl::FileApiImpl::default(),
                task_pool,
                sender,
                receiver,
            }),
        }
    }
}

impl FileSystem {
    pub fn attach_binary(&self, key: &'static str, path: &'static str) {
        bevy::utils::tracing::info!("attach binary");
        self.inner.read(path, |data| FsEvent::AttachBinary {
            key: key.to_string(),
            path: path.to_string(),
            data,
        });
    }
    pub fn read(&self, path: &'static str) {
        self.inner
            .read(path, |data| FsEvent::File(path.to_string(), data));
    }
    pub fn read_dir(&self, path: &'static str) {
        self.inner
            .read_dir(path, |files| FsEvent::FileList(path.to_string(), files));
    }
    pub fn write(&self, path: &str, contents: &[u8]) {
        let path2 = path.to_owned();
        self.inner
            .write(&path, contents.to_owned(), move |_| FsEvent::Written(path2));
    }
}

pub fn pump_fs_events(fs: Res<FileSystem>, mut events: EventWriter<FsEvent>) {
    for event in fs.inner.receiver.try_iter() {
        if let Some(event) = event {
            events.send(event);
        }
    }
}
