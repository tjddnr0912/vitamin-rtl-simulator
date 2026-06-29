//! P4-T1: the VCD writer THREAD behind `--threads ≥2`.
//!
//! The producer (simulation thread) keeps doing ALL deterministic work — VCD
//! encoding and record ordering are untouched — and hands finished byte chunks
//! to a dedicated writer thread over an order-preserving bounded FIFO. Because
//! the writer receives the exact byte stream the single-thread path would have
//! written, the output file is byte-identical for every thread count (the P4
//! contract: `--threads` changes wall-clock only). A slow disk applies
//! BACKPRESSURE through the bounded queue instead of growing memory.
//!
//! Error model: `write` is fire-and-forget (the producer's `BufWriter` batches
//! chunks; a per-chunk ack would serialize the pipeline away). The FIRST write
//! error is latched in the thread and surfaced at the next `flush` — exactly
//! where `finalize_vcd` looks, so the `W-RUN-VCD-WRITE-FAIL` diagnostic (P2-2)
//! works identically on the threaded path.

use std::io::Write;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

enum Msg {
    Bytes(Vec<u8>),
    /// Flush request: drain-or-latch error → flush inner → ack with the result.
    Flush(SyncSender<std::io::Result<()>>),
}

/// A `Write` adapter that forwards to a spawned writer thread owning the real
/// sink. Queue: 8 chunks (the producer batches ~64 KiB each ⇒ ≤ ~512 KiB in
/// flight). `Drop` closes the channel, lets the thread drain, and joins it.
pub(crate) struct ThreadedWriter {
    tx: Option<SyncSender<Msg>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl ThreadedWriter {
    pub(crate) fn spawn<W: Write + Send + 'static>(mut inner: W) -> Self {
        let (tx, rx): (SyncSender<Msg>, Receiver<Msg>) = sync_channel(8);
        // This thread performs NO recursive work — it only `write_all`s
        // pre-serialized byte chunks from the FIFO — so the OS default stack is
        // safe by design (unlike the recursive parse/elaborate pipeline, which
        // the driver runs on a large explicit stack; see crates/cli/src/main.rs).
        let handle = std::thread::spawn(move || {
            let mut first_err: Option<std::io::Error> = None;
            for msg in rx {
                match msg {
                    Msg::Bytes(b) => {
                        if first_err.is_none() {
                            if let Err(e) = inner.write_all(&b) {
                                first_err = Some(e);
                            }
                        }
                    }
                    Msg::Flush(ack) => {
                        let res = match first_err.take() {
                            Some(e) => Err(e),
                            None => inner.flush(),
                        };
                        let _ = ack.send(res);
                    }
                }
            }
            // Channel closed (producer dropped without a final explicit flush):
            // best-effort flush — `finalize_vcd` already surfaced any error.
            if first_err.is_none() {
                let _ = inner.flush();
            }
        });
        ThreadedWriter {
            tx: Some(tx),
            handle: Some(handle),
        }
    }
}

impl Write for ThreadedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(tx) = &self.tx {
            // A send error means the writer thread died; the latched error (if
            // any) surfaces at flush. Report success so the producer's BufWriter
            // doesn't retry-loop.
            let _ = tx.send(Msg::Bytes(buf.to_vec()));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let Some(tx) = &self.tx else {
            return Ok(());
        };
        let (ack_tx, ack_rx) = sync_channel(1);
        if tx.send(Msg::Flush(ack_tx)).is_err() {
            return Err(std::io::Error::other("VCD writer thread terminated"));
        }
        ack_rx
            .recv()
            .map_err(|_| std::io::Error::other("VCD writer thread terminated"))?
    }
}

impl Drop for ThreadedWriter {
    fn drop(&mut self) {
        drop(self.tx.take()); // close the channel → thread drains and exits
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Shared in-memory sink the writer thread can own (Send).
    #[derive(Clone, Default)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);
    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Bytes arrive complete and IN ORDER (the byte-identity foundation).
    #[test]
    fn passes_bytes_through_in_order() {
        let buf = SharedBuf::default();
        let mut w = ThreadedWriter::spawn(buf.clone());
        for i in 0..100u8 {
            w.write_all(&[i; 33]).unwrap();
        }
        w.flush().unwrap();
        let got = buf.0.lock().unwrap().clone();
        let want: Vec<u8> = (0..100u8).flat_map(|i| [i; 33]).collect();
        assert_eq!(got, want);
    }

    /// Drop without an explicit flush still drains everything (join on drop).
    #[test]
    fn drop_drains_pending_writes() {
        let buf = SharedBuf::default();
        {
            let mut w = ThreadedWriter::spawn(buf.clone());
            w.write_all(b"tail bytes").unwrap();
        } // drop → drain → join
        assert_eq!(buf.0.lock().unwrap().as_slice(), b"tail bytes");
    }

    /// A write error in the thread surfaces at the NEXT flush (where
    /// `finalize_vcd` looks for it).
    #[test]
    fn write_error_surfaces_at_flush() {
        struct FailSink;
        impl Write for FailSink {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut w = ThreadedWriter::spawn(FailSink);
        w.write_all(b"doomed").unwrap(); // deferred
        let err = w.flush().expect_err("flush must report the latched error");
        assert!(err.to_string().contains("disk full"), "got: {err}");
    }
}
