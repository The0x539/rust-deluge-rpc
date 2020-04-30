use std::io::Write;
use byteorder::{ReadBytesExt, WriteBytesExt, BE};
use serde::{ser, Serialize, de, Deserialize, ser::Error as SerErr, de::Error as DeErr};

macro_rules! pseudo_enum {
    (
        $(#![$g_attr:meta])*
        $vis:vis $name:ident: $type:ty;
        $(
            $(#[$attr:meta])*
            $variant:ident = $value:expr$(,)?
         )*
    ) => {
        #[allow(non_snake_case, non_upper_case_globals)]
        $vis mod $name {
            $(#![$g_attr])*
            $(
                $(#[$attr])*
                pub const $variant: $type = $value;
             )*
        }
    }
}

pseudo_enum! {
    types: u8;
    LIST = 59, DICT = 60,
    #[allow(dead_code)] INT = 61,
    INT1 = 62, INT2 = 63, INT4 = 64, INT8 = 65,
    FLOAT32 = 66, FLOAT64 = 44,
    TRUE = 67, FALSE = 68,
    NONE = 69,
    TERM = 127,
}

pub const INT_POS_START: i8 = 0;
pub const INT_POS_MAX: i8 = 43;

pub const INT_NEG_START: i8 = 70;
pub const INT_NEG_MIN: i8 = -32;

pub const STR_START: u8 = 128;
pub const STR_COUNT: usize = 64;
pub const STR_END: u8 = STR_START - 1 + STR_COUNT as u8;

pub const LIST_START: u8 = STR_START + STR_COUNT as u8;
pub const LIST_COUNT: usize = 64;
pub const LIST_END: u8 = LIST_START - 1 + LIST_COUNT as u8;

pub const DICT_START: u8 = 102;
pub const DICT_COUNT: usize = 25;
pub const DICT_END: u8 = DICT_START - 1 + DICT_COUNT as u8;

#[derive(Debug)]
pub struct Whatever(String);
impl std::fmt::Display for Whatever {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for Whatever {}
impl DeErr for Whatever {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self(format!("{}", msg))
    }
}
impl SerErr for Whatever {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Self(format!("{}", msg))
    }
}
type Result<T> = std::result::Result<T, Whatever>;

struct RencodeSerializer(Vec<u8>, Vec<usize>);

impl RencodeSerializer {
    // This is a Vec, so if these writes fail, we have bigger problems.
    fn write_all(&mut self, buf: &[u8]) { self.0.write_all(buf).unwrap(); }
    fn write_u8(&mut self, n: u8) { self.0.write_u8(n).unwrap(); }
    fn write_i8(&mut self, n: i8) { self.0.write_i8(n).unwrap(); }
    fn write_i16(&mut self, n: i16) { self.0.write_i16::<BE>(n).unwrap(); }
    fn write_i32(&mut self, n: i32) { self.0.write_i32::<BE>(n).unwrap(); }
    fn write_i64(&mut self, n: i64) { self.0.write_i64::<BE>(n).unwrap(); }
    fn write_f32(&mut self, n: f32) { self.0.write_f32::<BE>(n).unwrap(); }
    fn write_f64(&mut self, n: f64) { self.0.write_f64::<BE>(n).unwrap(); }
}

pub fn to_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut serializer = RencodeSerializer(vec![], vec![]);
    value.serialize(&mut serializer)?;
    serializer.0.flush().unwrap();
    Ok(serializer.0)
}

// Stuff I do need
impl<'a> ser::SerializeSeq for &'a mut RencodeSerializer {
    type Ok = ();
    type Error = Whatever;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<()> {
        v.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        if self.1.pop().unwrap() >= LIST_COUNT {
            self.write_u8(types::TERM);
        }
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut RencodeSerializer {
    type Ok = ();
    type Error = Whatever;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<()> {
        v.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        if self.1.pop().unwrap() >= LIST_COUNT {
            self.write_u8(types::TERM);
        }
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut RencodeSerializer {
    type Ok = ();
    type Error = Whatever;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<()> {
        v.serialize(&mut **self)
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<()> {
        v.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        if self.1.pop().unwrap() >= DICT_COUNT {
            self.write_u8(types::TERM);
        }
        Ok(())
    }
}

type Impossible = ser::Impossible<(), Whatever>;

impl<'a> ser::Serializer for &'a mut RencodeSerializer {
    type Ok = ();
    type Error = Whatever;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeMap = Self;

    type SerializeTupleStruct = Impossible;
    type SerializeTupleVariant = Impossible;
    type SerializeStruct = Impossible;
    type SerializeStructVariant = Impossible;

    fn serialize_unit(self) -> Result<()> {
        self.write_u8(types::NONE);
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<()> {
        v.serialize(self)
    }

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.write_u8(if v { types::TRUE } else { types::FALSE });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        match v {
            0..=INT_POS_MAX => {
                self.write_i8(INT_POS_START + v);
            }
            INT_NEG_MIN..=-1 => {
                self.write_i8(INT_NEG_START - 1 - v);
            }
            _ => {
                self.write_u8(types::INT1);
                self.write_i8(v);
            }
        }
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.write_u8(types::INT2);
        self.write_i16(v);
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.write_u8(types::INT4);
        self.write_i32(v);
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.write_u8(types::INT8);
        self.write_i64(v);
        Ok(())
    }
    
    fn serialize_f32(self, v: f32) -> Result<()> {
        self.write_u8(types::FLOAT32);
        self.write_f32(v);
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.write_u8(types::FLOAT64);
        self.write_f64(v);
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        let len = v.len();
        if len < STR_COUNT {
            self.write_u8(STR_START + len as u8);
        } else {
            self.write_all(format!("{}:", len).as_bytes());
        }
        self.write_all(v);
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.serialize_tuple(len.ok_or(SerErr::custom("try .collect()"))?)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        if len < LIST_COUNT {
            self.write_u8(LIST_START + len as u8);
        } else {
            self.write_u8(types::LIST);
        }
        self.1.push(len);
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        let len = len.ok_or(SerErr::custom("need to know map size ahead of time"))?;
        if len < DICT_COUNT {
            self.write_u8(DICT_START + len as u8);
        } else {
            self.write_u8(types::DICT);
        }
        self.1.push(len);
        Ok(self)
    }

    // Data types not supported by the real rencode
    fn serialize_char(self, _: char) -> Result<()> { unimplemented!() }
    fn serialize_u8(self, _: u8) -> Result<()> { unimplemented!() }
    fn serialize_u16(self, _: u16) -> Result<()> { unimplemented!() }
    fn serialize_u32(self, _: u32) -> Result<()> { unimplemented!() }
    fn serialize_u64(self, _: u64) -> Result<()> { unimplemented!() }
    fn serialize_struct(self, _: &str, _: usize) -> Result<Self::SerializeStruct> { unimplemented!() }
    fn serialize_struct_variant(self, _: &str, _: u32, _: &str, _: usize) -> Result<Self::SerializeStructVariant> { unimplemented!() }
    fn serialize_unit_struct(self, _: &str) -> Result<()> { unimplemented!() }
    fn serialize_unit_variant(self, _: &str, _: u32, _: &str) -> Result<()> { unimplemented!() }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _: &str, _: &T) -> Result<()> { unimplemented!() }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _: &str, _: u32, _: &str, _: &T) -> Result<()> { unimplemented!() }
    fn serialize_tuple_struct(self, _: &str, _: usize) -> Result<Self::SerializeTupleStruct> { unimplemented!() }
    fn serialize_tuple_variant(self, _: &str, _: u32, _: &str, _: usize) -> Result<Self::SerializeTupleVariant> { unimplemented!() }
}

struct RencodeDeserializer<'de>(&'de [u8], usize);

pub fn from_bytes<'a, T: Deserialize<'a>>(b: &'a [u8]) -> Result<T> {
    let mut deserializer = RencodeDeserializer(b, 0);
    let val = T::deserialize(&mut deserializer)?;
    if deserializer.1 != deserializer.0.len() {
        Ok(val)
    } else {
        Err(DeErr::custom("too many bytes"))
    }
}

impl<'de> RencodeDeserializer<'de> {
    fn peek_byte(&self) -> u8 {
        self.0[self.1]
    }

    fn next_byte(&mut self) -> u8 {
        let val = self.peek_byte();
        self.1 += 1;
        val
    }

    fn next_slice(&mut self, n: usize) -> &'de [u8] {
        let val = &self.0[self.1..self.1+n];
        self.1 += n;
        val
    }

    fn next_i8(&mut self) -> i8 { self.next_slice(1).read_i8().unwrap() }
    fn next_i16(&mut self) -> i16 { self.next_slice(2).read_i16::<BE>().unwrap() }
    fn next_i32(&mut self) -> i32 { self.next_slice(4).read_i32::<BE>().unwrap() }
    fn next_i64(&mut self) -> i64 { self.next_slice(8).read_i64::<BE>().unwrap() }

    fn next_f32(&mut self) -> f32 { self.next_slice(4).read_f32::<BE>().unwrap() }
    fn next_f64(&mut self) -> f64 { self.next_slice(8).read_f64::<BE>().unwrap() }

    fn next_str(&mut self, len: Option<usize>) -> &'de str {
        if let Some(len) = len {
            return std::str::from_utf8(self.next_slice(len)).unwrap();
        }
        self.1 -= 1; // bit of a terrible hack but whatever
        let mut splitn = self.0[self.1..].splitn(2, |&c| c == 58);
        let len_str: &str = std::str::from_utf8(splitn.next().unwrap()).unwrap();
        let len = len_str.parse::<usize>().unwrap();
        std::str::from_utf8(&splitn.next().unwrap()[0..len]).unwrap()
    }
}

struct FixedLengthSeq<'a, 'de: 'a>(&'a mut RencodeDeserializer<'de>, usize);

impl<'de, 'a> de::SeqAccess<'de> for FixedLengthSeq<'a, 'de> {
    type Error = Whatever;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.1 == 0 {
            return Ok(None);
        }
        self.1 -= 1;
        seed.deserialize(&mut *self.0).map(Some)
    }
}

struct FixedLengthMap<'a, 'de: 'a>(&'a mut RencodeDeserializer<'de>, usize, bool);

impl<'de, 'a> de::MapAccess<'de> for FixedLengthMap<'a, 'de> {
    type Error = Whatever;

    fn next_key_seed<T: de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.2 {
            panic!("tried to get a key inappropriately");
        }
        if self.1 == 0 {
            return Ok(None);
        }
        self.2 = true;
        seed.deserialize(&mut *self.0).map(Some)
    }

    fn next_value_seed<T: de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<T::Value> {
        if !self.2 {
            panic!("tried to get a value inappropriately");
        }
        self.1 -= 1;
        self.2 = false;
        seed.deserialize(&mut *self.0)
    }
}

struct TerminatedSeq<'a, 'de: 'a>(&'a mut RencodeDeserializer<'de>);

impl<'de, 'a> de::SeqAccess<'de> for TerminatedSeq<'a, 'de> {
    type Error = Whatever;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.0.peek_byte() == types::TERM {
            return Ok(None);
        }
        seed.deserialize(&mut *self.0).map(Some)
    }
}

struct TerminatedMap<'a, 'de: 'a>(&'a mut RencodeDeserializer<'de>, bool);

impl<'de, 'a> de::MapAccess<'de> for TerminatedMap<'a, 'de> {
    type Error = Whatever;

    fn next_key_seed<T: de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.1 {
            panic!("tried to get a key inappropriately");
        }
        if self.0.peek_byte() == types::TERM {
            return Ok(None);
        }
        self.1 = true;
        seed.deserialize(&mut *self.0).map(Some)
    }

    fn next_value_seed<T: de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<T::Value> {
        if !self.1 {
            panic!("tried to get a value inappropriately");
        }
        self.1 = false;
        seed.deserialize(&mut *self.0)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut RencodeDeserializer<'de> {
    type Error = Whatever;

    fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let x = self.next_byte();
        match x {
            types::NONE => visitor.visit_unit(),
            types::TRUE => visitor.visit_bool(true),
            types::FALSE => visitor.visit_bool(false),
            types::INT1 => visitor.visit_i8(self.next_i8()),
            types::INT2 => visitor.visit_i16(self.next_i16()),
            types::INT4 => visitor.visit_i32(self.next_i32()),
            types::INT8 => visitor.visit_i64(self.next_i64()),
            types::INT => unimplemented!(),
            
            types::FLOAT32 => visitor.visit_f32(self.next_f32()),
            types::FLOAT64 => visitor.visit_f64(self.next_f64()),

            0..=43 => visitor.visit_i8(INT_POS_START + x as i8),
            70..=101 => visitor.visit_i8(70 - 1 - x as i8),

            STR_START..=STR_END => visitor.visit_borrowed_str(self.next_str(Some(x as usize))),
            49..=57 => visitor.visit_borrowed_str(self.next_str(None)),
            58 => Err(DeErr::custom("unexpected strlen terminator")),

            LIST_START..=LIST_END => visitor.visit_seq(FixedLengthSeq(self, (x - LIST_START) as usize)),
            types::LIST => visitor.visit_seq(TerminatedSeq(self)),

            DICT_START..=DICT_END => visitor.visit_map(FixedLengthMap(self, (x - DICT_START) as usize, false)),
            types::DICT => visitor.visit_map(TerminatedMap(self, false)),

            types::TERM => Err(DeErr::custom("unexpected list/dict terminator")),

            45..=48 => Err(DeErr::custom("I don't know what values 45-48 are supposed to mean")),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}
