use std::io::{IoResult, IoError, InvalidInput};
use std::iter::count;
use std::num;

pub trait Varint: FromPrimitive + ToPrimitive {
    fn varlen(&self) -> uint;
}

pub trait VarintReader {
    fn read_le_varint<V: Varint>(&mut self) -> IoResult<V>;
}

pub trait VarintWriter {
    fn write_le_varint<V: Varint>(&mut self, x: V) -> IoResult<uint>;
}

fn varint_length(mut x: u64) -> uint {
	let mut i = 0;
	while x >= 0b1000_0000 {
		x >>= 7;
		i += 1;
	}
	i + 1
}

impl Varint for uint {
    fn varlen(&self) -> uint {
        varint_length(*self as u64)
    }
}

impl Varint for u8 {
    fn varlen(&self) -> uint {
        varint_length(*self as u64)
    }
}

impl Varint for u16 {
    fn varlen(&self) -> uint {
        varint_length(*self as u64)
    }
}

impl Varint for u32 {
    fn varlen(&self) -> uint {
        varint_length(*self as u64)
    }
}

impl Varint for u64 {
    fn varlen(&self) -> uint {
        varint_length(*self as u64)
    }
}

static OWERFLOW_ERROR: IoError = IoError {
    kind: InvalidInput,
    desc: "owerflow",
    detail: None,
};


impl<R> VarintReader for R where R: Reader {
    fn read_le_varint<V: Varint>(&mut self) -> IoResult<V> {
        read_le_varint(self).and_then(|x| {
            match num::from_u64(x) {
                Some(x) => Ok(x),
                None => Err(OWERFLOW_ERROR.clone()),
            }
        })
    }
}


fn read_le_varint<R: Reader>(reader: &mut R) -> IoResult<u64> {
    let mut x = 0u64;
    let mut shift = 0u;
    for i in count(0u, 1) {
        let b = try!(reader.read_byte());
        if b < 0b1000_0000 {
            if (i == 9 && b > 1) || i >= 10 {
                return Err(OWERFLOW_ERROR.clone())
            }
            return Ok(x | b as u64 << shift)
        }
        x |= (b as u64 & 0b0111_1111) << shift;
        shift += 7;
    }
    unreachable!();
}

impl<W> VarintWriter for W where W: Writer {
    fn write_le_varint<V: Varint>(&mut self, x: V) -> IoResult<uint> {
        write_le_varint(self, x.to_u64().unwrap())
    }
}

fn write_le_varint<W: Writer>(writer: &mut W, mut x: u64) -> IoResult<uint> {
    let mut i = 0;
    while x >= 0b1000_0000 {
        try!(writer.write_u8(x as u8 | 0b1000_0000));
        x >>= 7;
        i += 1;
    }
    try!(writer.write_u8(x as u8));
    Ok(i + 1)
}

#[cfg(test)]
mod test {
    use std::io::{BufReader, BufWriter, IoResult, OtherIoError};
    use std::fmt::Show;

    use super::{VarintReader, VarintWriter, Varint, OWERFLOW_ERROR};

    fn test_read_le_varint<V: Varint + PartialEq + Show>(buf: &[u8], expected: IoResult<V>) {
        let mut r = BufReader::new(buf);
        assert_eq!(r.read_le_varint(), expected);
    }

    #[test]
    fn read_le_varint() {
        test_read_le_varint([0x00], Ok(0x00u32));
        test_read_le_varint([0x7F], Ok(0x7Fu32));
        test_read_le_varint([0x80, 0x01], Ok(0x80u32));
        test_read_le_varint([0xAC, 0x02], Ok(300u32));
        test_read_le_varint([0x80, 0x01], Ok(0x80u8));
        test_read_le_varint([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01], Ok(0xFFFFFFFFFFFFFFFFu64));
    }

    #[test]
    fn read_le_varint_owerflow() {
        test_read_le_varint::<u8>([0xAC, 0x02], Err(OWERFLOW_ERROR.clone()));
        test_read_le_varint::<u64>([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F], Err(OWERFLOW_ERROR.clone()));
    }

    fn test_write_le_varint<V: Varint + PartialEq + Show>(x: V, result: &[u8]) {
        let mut buf = [0, ..10];
        let n = {
            let mut r = BufWriter::new(&mut buf);
            let r = r.write_le_varint(x);
            assert_eq!(Ok(result.len()), r);
            r.unwrap()
        };
        assert_eq!(buf[.. n], result);
    }

    #[test]
    fn write_le_varint() {
        test_write_le_varint(0x00u32, [0x00]);
        test_write_le_varint(0x7Fu32, [0x7F]);
        test_write_le_varint(0x80u32, [0x80, 0x01]);
        test_write_le_varint(300u32, [0xAC, 0x02]);
        test_write_le_varint(0x80u8, [0x80, 0x01]);
        test_write_le_varint(0xFFFFFFFFFFFFFFFFu64, [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]);
    }

    #[test]
    fn write_le_varint_erro_is_reported() {
        let mut buf = [0, ..1];
        let mut r = BufWriter::new(&mut buf);
        match r.write_le_varint(0x80u32) {
            Err(ref err) if err.kind == OtherIoError => {},
            res => fail!(format!("{}", res)),
        }
    }
}
