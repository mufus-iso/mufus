use sha2::{Digest, Sha256};
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, Ordering};

// This prototype is compiled only for tests.
fn write_and_verify(
    source: &mut impl Read,
    target: &mut (impl Read + Write + Seek),
    size: u64,
    capacity: u64,
    cancelled: &AtomicBool,
) -> io::Result<()> {
    if size == 0 || size > capacity {
        return Err(io::Error::other("invalid image size"));
    }
    target.seek(SeekFrom::Start(0))?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 8192];
    let mut remaining = size;
    while remaining > 0 {
        if cancelled.load(Ordering::Relaxed) {
            return Err(io::Error::other("cancelled"));
        }
        let count = usize::try_from(remaining.min(buffer.len() as u64))
            .map_err(|_| io::Error::other("invalid length"))?;
        source.read_exact(&mut buffer[..count])?;
        target.write_all(&buffer[..count])?;
        hash.update(&buffer[..count]);
        remaining -= count as u64;
    }
    if source.read(&mut buffer[..1])? != 0 {
        return Err(io::Error::other("image exceeds declared size"));
    }
    target.flush()?;
    target.seek(SeekFrom::Start(0))?;
    let mut readback = Sha256::new();
    remaining = size;
    while remaining > 0 {
        if cancelled.load(Ordering::Relaxed) {
            return Err(io::Error::other("cancelled"));
        }
        let count = usize::try_from(remaining.min(buffer.len() as u64))
            .map_err(|_| io::Error::other("invalid length"))?;
        target.read_exact(&mut buffer[..count])?;
        readback.update(&buffer[..count]);
        remaining -= count as u64;
    }
    if hash.finalize() != readback.finalize() {
        return Err(io::Error::other("verification failed"));
    }
    Ok(())
}

#[test]
fn rejects_corrupt_readback_and_mid_write_cancellation() {
    struct Corrupt(Cursor<Vec<u8>>);
    impl Read for Corrupt {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let count = self.0.read(buffer)?;
            if count > 0 {
                buffer[0] ^= 1;
            }
            Ok(count)
        }
    }
    impl Write for Corrupt {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.0.write(buffer)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
    impl Seek for Corrupt {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            self.0.seek(position)
        }
    }

    struct CancelAfterRead<'a> {
        data: &'a [u8],
        cancelled: &'a AtomicBool,
    }
    impl Read for CancelAfterRead<'_> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let count = self.data.read(buffer)?;
            self.cancelled.store(true, Ordering::Relaxed);
            Ok(count)
        }
    }
    let cancelled = AtomicBool::new(false);
    let data = vec![42; 9000];
    let mut corrupt = Corrupt(Cursor::new(Vec::new()));
    let error = write_and_verify(&mut data.as_slice(), &mut corrupt, 9000, 9000, &cancelled)
        .expect_err("corruption must fail verification");
    assert_eq!(error.to_string(), "verification failed");

    let mut source = CancelAfterRead {
        data: &data,
        cancelled: &cancelled,
    };
    let mut target = Cursor::new(Vec::new());
    let error = write_and_verify(&mut source, &mut target, 9000, 9000, &cancelled)
        .expect_err("cancellation must stop the write");
    assert_eq!(error.to_string(), "cancelled");
    assert_eq!(target.get_ref().len(), 8192);
}

#[test]
fn checks_size_cancellation_and_readback() {
    let cancelled = AtomicBool::new(false);
    let data = vec![42; 9000];
    let mut target = Cursor::new(Vec::new());
    assert!(write_and_verify(&mut data.as_slice(), &mut target, 9000, 9000, &cancelled).is_ok());
    assert_eq!(target.get_ref(), &data);
    assert!(write_and_verify(&mut data.as_slice(), &mut target, 9000, 8999, &cancelled).is_err());
    assert!(write_and_verify(&mut data.as_slice(), &mut target, 9001, 9001, &cancelled).is_err());
    assert!(write_and_verify(&mut data.as_slice(), &mut target, 8999, 9000, &cancelled).is_err());
    cancelled.store(true, Ordering::Relaxed);
    assert!(write_and_verify(&mut data.as_slice(), &mut target, 9000, 9000, &cancelled).is_err());
}
