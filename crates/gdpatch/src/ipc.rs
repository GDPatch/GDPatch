//! Handles sending messages between the Rust component and GDScript autoload.
use crate::{GDPatch, mods::ModInfo};
use filesilly::Stream;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::{Cursor, Read, Seek, Write},
    path::PathBuf,
};

pub const IPC_FILENAME: &str = "gdpatch-ipc";

#[derive(Serialize, Deserialize)]
pub struct Sequenced<T> {
    #[serde(default)]
    seq: usize,

    #[serde(flatten)]
    data: T,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum IpcCommand {
    GetModList,
    GetRootDirectory,
    GetModDirectory {
        mod_id: String,
    },
    GetConfigOption {
        mod_id: String,
        section: String,
        option: String,
    },
    SetConfigOption {
        mod_id: String,
        section: String,
        option: String,
        value: Option<toml::Value>,
    },
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum IpcResponse {
    ModList { value: Vec<ModInfo> },
    RootDirectory { value: PathBuf },
    ModDirectory { value: Option<PathBuf> },
    ConfigOption { value: Option<toml::Value> },
}

pub struct IpcStream {
    queued_messages: VecDeque<Vec<u8>>,
    read_buffer: Option<Cursor<Vec<u8>>>,
    write_buffer: Cursor<Vec<u8>>,
}

impl IpcStream {
    pub fn new() -> Self {
        Self {
            queued_messages: VecDeque::new(),
            read_buffer: None,
            write_buffer: Cursor::new(Vec::new()),
        }
    }

    fn process_command(&mut self, str: &str) -> color_eyre::Result<()> {
        let command: Sequenced<IpcCommand> = serde_json::from_str(str)?;
        let seq = command.seq;

        match command.data {
            IpcCommand::GetModList => {
                let gdpatch = GDPatch::instance();
                let mods = gdpatch.mods.read();
                let mods = mods.as_ref().expect("mods should be initialized");
                let mod_infos = mods.0.values().map(|m| m.info.clone()).collect();
                self.submit_response(seq, IpcResponse::ModList { value: mod_infos })?;
            }
            IpcCommand::GetRootDirectory => {
                let gdpatch = GDPatch::instance();
                let root_directory = gdpatch.get_root_directory();
                self.submit_response(
                    seq,
                    IpcResponse::RootDirectory {
                        value: root_directory,
                    },
                )?;
            }
            IpcCommand::GetModDirectory { mod_id } => {
                let gdpatch = GDPatch::instance();
                let mod_directory = gdpatch.get_mod_directory(&mod_id);
                self.submit_response(
                    seq,
                    IpcResponse::ModDirectory {
                        value: mod_directory,
                    },
                )?;
            }
            IpcCommand::GetConfigOption {
                mod_id,
                section,
                option,
            } => {
                let gdpatch = GDPatch::instance();
                let value = gdpatch.get_config_option(&mod_id, &section, &option);
                self.submit_response(seq, IpcResponse::ConfigOption { value })?;
            }
            IpcCommand::SetConfigOption {
                mod_id,
                section,
                option,
                value,
            } => {
                let gdpatch = GDPatch::instance();
                gdpatch.set_config_option(&mod_id, &section, &option, value)?;
            }
        }

        Ok(())
    }

    fn submit_response(&mut self, seq: usize, data: IpcResponse) -> color_eyre::Result<()> {
        let data = Sequenced::<IpcResponse> { seq, data };
        let mut str = serde_json::to_vec(&data)?;
        str.push(b'\n');
        self.queued_messages.push_back(str);

        Ok(())
    }
}

impl Read for IpcStream {
    // this is some ass
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.read_buffer.is_none()
            && let Some(bytes) = self.queued_messages.pop_front()
        {
            let cursor = Cursor::new(bytes);
            self.read_buffer = Some(cursor);
        }

        let cursor = match &mut self.read_buffer {
            Some(cursor) => cursor,
            None => return Ok(0),
        };

        let res = cursor.read(buf);
        if cursor.position() == cursor.stream_len()? {
            self.read_buffer = None;
        }

        res
    }
}

impl Write for IpcStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let res = self.write_buffer.write(buf);

        let buffer = self.write_buffer.get_ref();
        if let Ok(str) = str::from_utf8(buffer)
            && let Some(pos) = str.find('\n')
        {
            let line = &str[..pos];
            let line = line.to_owned();

            if let Err(err) = self.process_command(&line) {
                tracing::error!(err = %err, line, "failed to process ipc command");
            }

            self.write_buffer = Cursor::new(Vec::new());
        }

        res
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for IpcStream {
    fn seek(&mut self, _pos: std::io::SeekFrom) -> std::io::Result<u64> {
        Ok(0)
    }
}

impl Stream for IpcStream {}
