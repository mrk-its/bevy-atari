use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::time::Duration;

use bevy::prelude::{info, warn, Plugin};

use crate::messages::{send_message, Message};
use crate::BreakPoint;
use gdbstub::common::Signal;
use gdbstub::conn::{Connection, ConnectionExt};
use gdbstub::stub::{run_blocking, SingleThreadStopReason};
use gdbstub::target::ext::base::{
    singlethread::{SingleThreadBase, SingleThreadResume, SingleThreadResumeOps},
    BaseOps,
};
use gdbstub::target::ext::host_io::{
    HostIoErrno, HostIoError, HostIoOpenFlags, HostIoOpenMode, HostIoResult,
};
use gdbstub::target::{self, Target, TargetError, TargetResult};
use gdbstub_mos_arch::{MOSArch, MosBreakpointKind, MosRegs};

use std::io;
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
pub enum GdbMessage {
    Registers(MosRegs),
    Memory(u16, Vec<u8>),
    Paused,
}

pub type GdbSender = Sender<GdbMessage>;
pub struct GdbChannel(pub GdbSender);

#[derive(Debug)]
enum TargetState {
    Paused,
    Running,
}

struct Emu {
    files: HashMap<u32, InMemoryFile>,
    receiver: Receiver<GdbMessage>,
    state: TargetState,
}

impl Emu {
    #[allow(dead_code)]
    fn new(receiver: Receiver<GdbMessage>) -> Self {
        Self {
            files: Default::default(),
            receiver,
            state: TargetState::Running,
        }
    }
    fn wait_for_stop(&mut self) {
        if let &TargetState::Paused = &self.state {
            info!("target already stopped");
            return;
        }
        loop {
            if let Ok(msg) = self.receiver.recv_timeout(Duration::from_millis(1000)) {
                if let GdbMessage::Paused = msg {
                    info!("target is paused");
                    self.state = TargetState::Paused;
                    return;
                } else {
                    warn!("unexpected message: {:?}", msg);
                }
            }
        }
    }
}

impl InMemoryFiles for Emu {
    fn get_files(&mut self) -> &mut HashMap<u32, InMemoryFile> {
        return &mut self.files;
    }
}

trait InMemoryFiles {
    fn get_files(&mut self) -> &mut HashMap<u32, InMemoryFile>;
}

impl Target for Emu {
    type Arch = MOSArch;
    type Error = &'static str;

    #[inline(always)]
    fn base_ops(&mut self) -> gdbstub::target::ext::base::BaseOps<'_, Self::Arch, Self::Error> {
        BaseOps::SingleThread(self)
    }

    #[inline(always)]
    fn support_breakpoints(
        &mut self,
    ) -> Option<target::ext::breakpoints::BreakpointsOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_host_io(&mut self) -> Option<target::ext::host_io::HostIoOps<'_, Self>> {
        Some(self)
    }
}

impl SingleThreadBase for Emu {
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as gdbstub::arch::Arch>::Registers,
    ) -> gdbstub::target::TargetResult<(), Self> {
        send_message(Message::ReadRegisters);
        let reply = self.receiver.recv().unwrap(); // TODO
        if let GdbMessage::Registers(mos_regs) = reply {
            *regs = mos_regs;
            Ok(())
        } else {
            Err(TargetError::Fatal("wrong reply, MosRegs expected"))
        }
    }

    fn write_registers(
        &mut self,
        _regs: &<Self::Arch as gdbstub::arch::Arch>::Registers,
    ) -> gdbstub::target::TargetResult<(), Self> {
        todo!()
    }

    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        data: &mut [u8],
    ) -> gdbstub::target::TargetResult<(), Self> {
        send_message(Message::ReadMemory(start_addr, data.len() as u16));
        let reply = self.receiver.recv().unwrap(); // TODO
        if let GdbMessage::Memory(offs, memory) = reply {
            assert!(start_addr == offs);
            data.copy_from_slice(&memory);
            Ok(())
        } else {
            Err(TargetError::Fatal("wrong reply, Memory expected"))
        }
    }

    fn write_addrs(
        &mut self,
        _start_addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        _data: &[u8],
    ) -> gdbstub::target::TargetResult<(), Self> {
        todo!()
    }

    #[inline(always)]
    fn support_resume(&mut self) -> Option<SingleThreadResumeOps<'_, Self>> {
        Some(self)
    }
}

impl target::ext::breakpoints::Breakpoints for Emu {
    #[inline(always)]
    fn support_sw_breakpoint(
        &mut self,
    ) -> Option<target::ext::breakpoints::SwBreakpointOps<'_, Self>> {
        Some(self)
    }
}

impl target::ext::breakpoints::SwBreakpoint for Emu {
    fn add_sw_breakpoint(
        &mut self,
        addr: u16,
        _kind: MosBreakpointKind,
    ) -> TargetResult<bool, Self> {
        send_message(Message::AddBreakpoint(BreakPoint::PC(addr)));
        Ok(true)
    }

    fn remove_sw_breakpoint(
        &mut self,
        addr: u16,
        _kind: MosBreakpointKind,
    ) -> TargetResult<bool, Self> {
        send_message(Message::DelBreakpoint(BreakPoint::PC(addr)));
        Ok(true)
    }
}

impl SingleThreadResume for Emu {
    fn resume(&mut self, signal: Option<Signal>) -> Result<(), Self::Error> {
        // Upon returning from the `resume` method, the target being debugged should be
        // configured to run according to whatever resume actions the GDB client has
        // specified (as specified by `set_resume_action`, `resume_range_step`,
        // `reverse_{step, continue}`, etc...)
        //
        // In this basic `armv4t` example, the `resume` method simply sets the exec mode
        // of the emulator's interpreter loop and returns.
        //
        // In more complex implementations, it's likely that the target being debugged
        // will be running in another thread / process, and will require some kind of
        // external "orchestration" to set it's execution mode (e.g: modifying the
        // target's process state via platform specific debugging syscalls).

        if signal.is_some() {
            return Err("no support for continuing with signal");
        }

        send_message(Message::Continue);

        // self.exec_mode = ExecMode::Continue;
        Ok(())
    }

    #[inline(always)]
    fn support_single_step(
        &mut self,
    ) -> Option<target::ext::base::singlethread::SingleThreadSingleStepOps<'_, Self>> {
        Some(self)
    }

    // #[inline(always)]
    // fn support_range_step(
    //     &mut self,
    // ) -> Option<target::ext::base::singlethread::SingleThreadRangeSteppingOps<'_, Self>> {
    //     Some(self)
    // }
}

impl target::ext::base::singlethread::SingleThreadSingleStep for Emu {
    fn step(&mut self, _signal: Option<Signal>) -> Result<(), Self::Error> {
        send_message(Message::SingleStep);
        Ok(())
    }
}

impl target::ext::host_io::HostIo for Emu {
    #[inline(always)]
    fn support_open(&mut self) -> Option<target::ext::host_io::HostIoOpenOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_close(&mut self) -> Option<target::ext::host_io::HostIoCloseOps<'_, Self>> {
        Some(self)
    }

    #[inline(always)]
    fn support_pwrite(&mut self) -> Option<target::ext::host_io::HostIoPwriteOps<'_, Self>> {
        Some(self)
    }
}

impl target::ext::host_io::HostIoOpen for Emu {
    fn open(
        &mut self,
        filename: &[u8],
        _flags: HostIoOpenFlags,
        _mode: HostIoOpenMode,
    ) -> HostIoResult<u32, Self> {
        let new_fd = self.get_files().keys().min().unwrap_or(&0) + 1;
        let path =
            std::str::from_utf8(filename).map_err(|_| HostIoError::Errno(HostIoErrno::ENOENT))?;

        let file = InMemoryFile::new(path.to_string());
        self.get_files().insert(new_fd, file);
        Ok(new_fd)
    }
}

impl target::ext::host_io::HostIoClose for Emu {
    fn close(&mut self, fd: u32) -> HostIoResult<(), Self> {
        let file = self.get_files().get_mut(&fd);
        if let Some(file) = file {
            send_message(Message::BinaryData {
                key: "xex".to_string(),
                path: "/".to_string(),
                data: Some(file.data.clone()),
                slot: None,
            });
            send_message(Message::Reset {
                cold: true,
                disable_basic: true,
            });
            self.state = TargetState::Running;
            send_message(Message::ClearBreakpoints);
            send_message(Message::AddBreakpoint(BreakPoint::IndirectPC(0x2e0)));
            self.wait_for_stop();
        }
        Ok(())
    }
}

impl target::ext::host_io::HostIoPwrite for Emu {
    fn pwrite(&mut self, fd: u32, _offset: u16, data: &[u8]) -> HostIoResult<u16, Self> {
        let file = self.get_files().get_mut(&fd);
        if let Some(file) = file {
            file.data.extend(data.iter());
        }
        Ok(data.len() as u16)
    }
}

enum MyGdbBlockingEventLoop {}

impl run_blocking::BlockingEventLoop for MyGdbBlockingEventLoop {
    type Target = Emu;
    type Connection = Box<dyn ConnectionExt<Error = std::io::Error>>;
    type StopReason = SingleThreadStopReason<u16>;

    fn wait_for_stop_reason(
        target: &mut Self::Target,
        conn: &mut Self::Connection,
    ) -> Result<
        run_blocking::Event<Self::StopReason>,
        run_blocking::WaitForStopReasonError<
            <Self::Target as Target>::Error,
            <Self::Connection as Connection>::Error,
        >,
    > {
        info!("waiting for stop");
        let reason = loop {
            if let Ok(GdbMessage::Paused) =
                target.receiver.recv_timeout(Duration::from_secs_f32(0.2))
            {
                break Ok(run_blocking::Event::TargetStopped(
                    SingleThreadStopReason::Signal(Signal::SIGTRAP),
                ));
            }
            if let Ok(Some(byte)) = conn.peek() {
                break Ok(run_blocking::Event::IncomingData(byte));
            }
        };
        info!("stopped");
        reason
    }

    fn on_interrupt(
        _target: &mut Self::Target,
    ) -> Result<Option<Self::StopReason>, <Self::Target as Target>::Error> {
        send_message(Message::Pause);
        Ok(None)
    }
}

#[allow(dead_code)]
fn wait_for_gdb_connection(port: u16) -> io::Result<TcpStream> {
    let sockaddr = format!("0.0.0.0:{}", port);
    info!("Waiting for a GDB connection on {:?}...", sockaddr);
    let sock = TcpListener::bind(sockaddr)?;
    let (stream, addr) = sock.accept()?;

    // Blocks until a GDB client connects via TCP.
    // i.e: Running `target remote localhost:<port>` from the GDB prompt.

    info!("Debugger connected from {}", addr);
    Ok(stream) // `TcpStream` implements `gdbstub::Connection`
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init(receiver: Receiver<GdbMessage>) {
    let mut target = Emu::new(receiver);

    std::thread::spawn(move || loop {
        if let Ok(stream) = wait_for_gdb_connection(9001) {
            let conn: Box<dyn ConnectionExt<Error = std::io::Error>> = Box::new(stream);
            let debugger = gdbstub::stub::GdbStub::new(conn);
            let result = debugger.run_blocking::<MyGdbBlockingEventLoop>(&mut target);
            info!("disconnect reason: {:?}", result);
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub fn init(_receiver: Receiver<GdbMessage>) {}

#[derive(Debug)]
pub struct InMemoryFile {
    pub filename: String,
    pub data: Vec<u8>,
}

impl InMemoryFile {
    pub fn new(filename: String) -> Self {
        Self {
            filename,
            data: vec![],
        }
    }
}

#[derive(Default)]
pub struct GdbPlugin;

impl Plugin for GdbPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let gdb_channel = GdbChannel(sender);
        app.insert_resource(gdb_channel);
        init(receiver);
    }
}
