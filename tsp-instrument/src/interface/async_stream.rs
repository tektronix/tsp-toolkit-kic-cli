use std::{
    io::{ErrorKind, Read, Write},
    rc::Rc,
    sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use crate::error::{InstrumentError, Result};

use crate::interface::{Interface, NonBlock};

//create an Async version of the interface
pub struct AsyncStream {
    join: JoinHandle<Result<Arc<dyn Interface + Send + Sync>>>,
    write_to: Sender<Vec<u8>>,
    read_from: Rc<Receiver<Vec<u8>>>,
    buffer: Vec<u8>,
    nonblocking: bool,
}

impl TryFrom<Arc<dyn Interface + Send + Sync>> for AsyncStream {
    type Error = InstrumentError;

    fn try_from(
        mut socket: Arc<dyn Interface + Send + Sync>,
    ) -> std::result::Result<Self, Self::Error> {
        let (write_to, read_into) = mpsc::channel();
        let (write_out, read_from) = mpsc::channel();
        let builder =
            std::thread::Builder::new().name("Instrument Communication Thread".to_string());
        //TODO: Populate name with instrument information

        let join = builder.spawn(move || -> Result<Arc<dyn Interface + Send + Sync>> {
            Arc::get_mut(&mut socket).unwrap().set_nonblocking(true)?;
            let read_into: Receiver<Vec<u8>> = read_into;
            let write_out: Sender<Vec<u8>> = write_out;

            'rw_loop: loop {
                // see if the application has anything to send to the instrument.
                std::thread::sleep(Duration::from_millis(1));
                match read_into.try_recv() {
                    // It does, so send it
                    Ok(msg) => {
                        let chunk_size = 1024;
                        let mut start = 0;
                        while start < msg.len() {
                            let end = std::cmp::min(
                                start.checked_add(chunk_size).unwrap_or(usize::MAX),
                                msg.len(),
                            );
                            let chunk = &msg[start..end];
                            let mut bytes_sent = 0;
                            loop {
                                match Arc::get_mut(&mut socket)
                                    .unwrap()
                                    // Do NOT add a newline here. It is added elsewhere.
                                    .write(&chunk[bytes_sent..])
                                {
                                    Ok(0) => {
                                        // All data has been sent
                                        break;
                                    }
                                    Ok(n) => {
                                        // Successfully sent some data
                                        bytes_sent =
                                            bytes_sent.checked_add(n).unwrap_or(usize::MAX);
                                    }
                                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                        // Non-blocking write would block
                                        // You can do other work here, like processing other tasks or sleeping
                                        std::thread::sleep(Duration::from_millis(1));
                                    }
                                    Err(e) => {
                                        // There was an Error sending to the instrument.
                                        // clean up and get out.
                                        return Err(e.into());
                                    }
                                }
                            }
                            start = start.checked_add(chunk_size).unwrap_or(msg.len());
                        }
                    }

                    Err(e) => match e {
                        // The sender has disconnected, therefore we need to clean up
                        mpsc::TryRecvError::Disconnected => break 'rw_loop,
                        mpsc::TryRecvError::Empty => {}
                    },
                }
                let buf = &mut [0u8; 512];
                if let Ok(size) = Arc::get_mut(&mut socket).unwrap().read(buf) {
                    let buf = &buf[..size];
                    // This `send()` sends this to a receiver that will be activated next time a self.read() is called.
                    if size > 0 && write_out.send(buf.to_vec()).is_err() {
                        return Err(std::io::Error::new(
                            ErrorKind::ConnectionReset,
                            "attempted to send message from device, but client was closed"
                                .to_string(),
                        )
                        .into());
                    }
                }
            }
            Ok(socket)
        })?;

        Ok(Self {
            join,
            write_to,
            read_from: Rc::new(read_from),
            buffer: Vec::new(),
            nonblocking: true,
        })
    }
}

impl Read for AsyncStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // define an error since all of the returned errors are for the same reason.
        let error: std::io::Error = std::io::Error::new(
            ErrorKind::NotConnected,
            "attempted to read asynchronously from socket, but it was not connected".to_string(),
        );

        let resp = if self.nonblocking {
            match self.read_from.try_recv() {
                Ok(resp) => resp,
                Err(e) => match e {
                    TryRecvError::Empty => Vec::default(),
                    TryRecvError::Disconnected => {
                        return Err(error);
                    }
                },
            }
        } else {
            if !self.buffer.is_empty() {
                let read_size = self.buffer.take(buf.len() as u64).read(buf)?;
                self.buffer = self.buffer[read_size..].into();
                return Ok(read_size);
            }
            match self.read_from.recv() {
                Ok(resp) => resp,
                Err(_) => {
                    return Err(error);
                }
            }
        };

        let _ = self.buffer.write(&resp);
        let read_size = self.buffer.take(buf.len() as u64).read(buf)?;
        self.buffer = self.buffer[read_size..].into();

        Ok(read_size)
    }
}

impl Write for AsyncStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let Ok(()) = self.write_to.send(Vec::from(buf)) else {
            return Err(std::io::Error::new(
                ErrorKind::NotConnected,
                "attempted to write asynchronously to socket, but it was not connected".to_string(),
            ));
        };

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl NonBlock for AsyncStream {
    fn set_nonblocking(&mut self, nonblocking: bool) -> Result<()> {
        self.nonblocking = nonblocking;
        Ok(())
    }
}

impl TryFrom<AsyncStream> for Arc<dyn Interface + Send + Sync> {
    type Error = InstrumentError;

    fn try_from(async_stream: AsyncStream) -> std::result::Result<Self, Self::Error> {
        drop(async_stream.write_to);
        match async_stream.join.join() {
            Ok(Ok(stream)) => Ok(stream),
            _ => Err(InstrumentError::ConnectionError {
                details: "unable to retrieve synchronous stream".to_string(),
            }),
        }
    }
}

impl Interface for AsyncStream {}
