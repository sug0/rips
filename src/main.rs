#[allow(dead_code)]
mod ips;

use std::env;
use std::process;
use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write, BufReader, BufWriter};

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        process::exit(1)
    }
}

#[inline]
fn run() -> io::Result<()> {
    const BUFSIZ: usize = 4096;

    let args: Vec<_> = env::args()
        .collect();

    if args.len() != 4 {
        let e = format!("Usage: {} <source> <patch> <destiny>", args[0]);
        return Err(io::Error::new(io::ErrorKind::Other, e))
    };

    // the arguments
    let (mut patch, mut dst) = {
        let (p_src, p_patch, p_dst) = (&args[1], &args[2], &args[3]);

        // open patch for reading
        let patch = BufReader::new(File::open(p_patch)?);
        
        // open file to be patched
        let mut src = BufReader::new(File::open(p_src)?);

        // open destiny file
        let mut dst = BufWriter::new(File::create(p_dst)?);

        // clone file to dst
        io::copy(&mut src, &mut dst)?;

        (patch, dst)
    };

    // patch the file
    let recs = ips::RecordIterator::new_with_bufsize(&mut patch, BUFSIZ);
    let mut rle = ips::mem::Owner::new(0);

    for record in recs {
        dst.seek(SeekFrom::Start(record.off()))?;
        match record.data() {
            ips::Data::Chunk(mut chk) => {
                dst.write(chk.get().unwrap())?;
            },
            ips::Data::RLE { byte, mut size } => {
                let pow = lower_power_of_two(size);

                // fetch buffer for RLE operations
                let mut buf_borrow = rle.slice_mut(..pow);
                let buf = buf_borrow.get().unwrap();

                // perform memset
                for i in buf.iter_mut() {
                    *i = byte
                }

                // write data in chunks
                while size > pow {
                    dst.write(buf)?;
                    size -= pow;
                }

                // write remaining data byte by byte
                while size != 0 {
                    dst.write(&buf[..1])?;
                    size -= 1;
                }
            }
        }
    }

    Ok(())
}

#[inline]
fn lower_power_of_two(mut x: usize) -> usize {
    x = x | (x >> 1);
    x = x | (x >> 2);
    x = x | (x >> 4);
    x = x | (x >> 8);
    x = x | (x >> 16);
    x - (x >> 1)
}
