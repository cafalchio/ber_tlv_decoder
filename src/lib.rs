use pyo3::prelude::*;
use std::io::Read;
use flate2::read::GzDecoder;
use std::collections::HashMap;


const CLASS_SHIFT: u8 = 6;
const TWO_BIT_MASK: u8 = 3;
const ENCODE_SHIFT: u8 = 5;
const MODULO_2: u8 = 1;
const CLASSNUM_MASK: u8 = 31;
const MASK_BIT7: u8 = 127;
const SHIFT_7: u8 = 7;
const HIGH_CLASS_NUM: u8 = 31;
const MASK_BIT8: u8 = 128;
const SHIFT_8: u8 = 8;

#[allow(non_snake_case)]
pub struct BerClass {
    pub UNIVERSAL: u8, 
    pub APPLICATION: u8,
    pub CONTEXT: u8,
    pub PRIVATE: u8
}


pub const BERCLASS: BerClass = BerClass {
    UNIVERSAL: 0,
    APPLICATION: 1,
    CONTEXT: 2,
    PRIVATE: 3,
};


const EOC: &[(u8, bool, u32, usize)] = &[
    (0, false, 0, 0),
    (2, true, 1, 0),
];


#[derive(Clone)]
#[pyclass]
pub struct BerTag {
    #[pyo3(get)]
    pub name: String, 
    #[pyo3(get)]
    pub tag_class: u8,
    #[pyo3(get)]
    pub constructed: bool,
    #[pyo3(get)]
    pub number: u32,
}

#[derive(Clone)] 
#[pyclass]
pub struct TlvObject {
    #[pyo3(get)]
    pub tag: BerTag, 
    #[pyo3(get)]
    pub length: usize,
    #[pyo3(get)]
    pub value: Vec<u8>,
    #[pyo3(get)]
    pub offset: usize,
    #[pyo3(get)]
    pub children: Vec<TlvObject>,
}

#[pymethods]
impl TlvObject {
    #[new]
    fn new(
        tag: BerTag, 
        length: usize, 
        value: Vec<u8>, 
        offset: usize, 
        children: Vec<TlvObject>
    ) -> Self {
        TlvObject {
            tag,
            length,
            value,
            offset,
            children,
        }
    }
}


pub struct Data<'a> {
    pub data: &'a [u8],
    pub point: u32,
}


impl<'a> Data<'a> {
    fn read(&mut self, size: usize) -> &[u8] {
        let start = self.point as usize;
        let end = start + size as usize;

        if end > self.data.len() {
            return &self.data[start..]; 
        }

        let res = &self.data[start..end];
        self.point += size as u32;
        res
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    hex::encode(bytes)
}


pub fn read_tag(stream: &mut Data) -> Option<Vec<u8>> {
    let buffer: &[u8] = stream.read(1);
    if buffer == &[] as &[u8] {
        return None;
    }
    let mut tag_bytes: Vec<u8> = vec![buffer[0]];

    if buffer[0] & HIGH_CLASS_NUM == HIGH_CLASS_NUM {
        loop {
            let buffer = stream.read(1);
            if buffer == &[] as &[u8] || buffer[0] & MASK_BIT8 == 0 {
                break;
            }
            tag_bytes.push(buffer[0]);
        }
    }
    Some(tag_bytes)
}


pub fn decode_tag(tag_bytes: &[u8]) -> BerTag{
    if tag_bytes.is_empty() {
        panic!("Empty tag -> Error decoding tag")
    }
    let first_byte: u8 = tag_bytes[0];
    let name: String = hex_encode(&tag_bytes[..1]);
    let tag_class: u8 = (first_byte >> CLASS_SHIFT) & TWO_BIT_MASK;
    let constructed: bool = ((first_byte >> ENCODE_SHIFT) & MODULO_2) != 0;
    let mut number: u32 = (first_byte & CLASSNUM_MASK) as u32;

    if number as u8 == CLASSNUM_MASK {
        number = 0;
        for &b in &tag_bytes[1..] {
            number = (number << SHIFT_7) | (b & MASK_BIT7) as u32;
            if b & MASK_BIT8 == 0 { // Stop condition when high bit is 0
                break;
            }
        }
    }
    BerTag {
        name: name,
        tag_class: tag_class,
        constructed: constructed,
        number: number,
    }
}


pub fn reached_eoc(tag: &BerTag, length: usize) -> bool {
    EOC.iter().any(|&(class, constructed, number, len)| 
        (class, constructed, number, len) == (tag.tag_class, tag.constructed, tag.number, length)
    )
}


pub fn length_size(stream: &mut Data) -> usize {
    let bytes_read = stream.read(1);
    if bytes_read == &[] as &[u8] {
        return 0;
    }
    else if  bytes_read[0] >> SHIFT_7 == 0 {
        return 1;
    } else {
        return (bytes_read[0] & MASK_BIT7) as usize;
    }
}


pub fn read_length(stream: &mut Data) -> (usize, u8) {

    let first_byte_opt = stream.read(1).get(0).copied();
    let first_byte: u8 = match first_byte_opt {
        Some(byte) => byte,
        None => panic!("Unexpected end of stream"),
    };
    if first_byte >> SHIFT_7 == 0 {
        return (first_byte.into(), 1);
    }
    let length_size: u8 = first_byte & MASK_BIT7;
    if length_size == 0 {
        return (0, 1);
    }
    let length_bytes: &[u8] = stream.read(length_size as usize);

    if length_bytes.len() != length_size as usize {
        panic!("Unexpected end of length")
    }
    
    let mut length: usize = 0;
    for &b in length_bytes.iter() {
        length = (length << SHIFT_8) | b as usize;
    }

    (length, length_size + 1)
}


pub fn decode_tlv(
    stream: &mut Data,
    mut offset: usize,
    depth: usize ,
) -> Option<TlvObject> {
   
    if offset >= stream.data.len() {
        return None;
    }
    let start_offset = offset;
    let tag_bytes: Vec<u8> = read_tag(stream)?;
    let tag: BerTag = decode_tag(&tag_bytes);
    let (length, length_size) = read_length(stream);
    // println!("depth {}: lenght {} ", depth, length);
    offset += tag_bytes.len() + length_size as usize;

    let data_len = stream.data.len();

    let value: &[u8] = stream.read(length);
    // println!("stream: {:?}", value);
    
    if reached_eoc(&tag, length) {
        return decode_tlv(stream, offset, depth);
    }
    
    if offset + length > data_len {
        return None;
    }
    
    let mut tlv = TlvObject {
        tag,
        length,
        value: Vec::from(value),
        offset: start_offset,
        children: vec![],
    };

    // create a new byte to represent stream
    let mut new_value = Data {
        data: &value,
        point: 0,
    };
    // println!("{}: {}", tlv.tag.name, hex_encode(&tlv.value));
    
    if tlv.tag.constructed {
        let mut child_offset: usize = 0;
        while child_offset < length as usize {
            if let Some(child) = decode_tlv(&mut new_value, child_offset, depth + 1) {
                child_offset += child.length as usize;
                tlv.children.push(child);
                // println!("{}", depth);
            } else {
                break;
            }
        }
    } 
    Some(tlv)
}



fn extract_tags(tlv: &TlvObject, tags: &mut HashMap<String, Vec<u8>>) {
    if tlv.children.is_empty() {
        let tag_name = tlv.tag.name.clone();
        let tag_value = tlv.value.clone();
        tags.insert(tag_name, tag_value); 
    } else {
        for child in &tlv.children {
            extract_tags(child, tags);
        }
    }
}


// ==================== Python bindings ========================

#[pyfunction]
fn tlv_from_gz_file(path: String) -> PyResult<Vec<HashMap<String, Vec<u8>>>> {
    let bytes = std::fs::read(path).unwrap();
    let mut gz = GzDecoder::new(&bytes[..]);
    let mut file_bytes = Vec::new();
    gz.read_to_end(&mut file_bytes).unwrap();

    let mut stream: Data = Data {
        data: &file_bytes[..],
        point: 0,
    };
    let mut tlvs: Vec<HashMap<String, Vec<u8>>> = Vec::new(); 

    while stream.data.len() > stream.point as usize {
        if let Some(tlv) = decode_tlv(&mut stream, 0, 0) {
            let mut tags: HashMap<String, Vec<u8>> = HashMap::new();
            extract_tags(&tlv, &mut tags);
            tlvs.push(tags);
            // break;
        } 
    }
    Ok(tlvs)
}

#[pymodule]
fn ber_tlv_decoder(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // m.add_function(wrap_pyfunction!(tlv_from_gz_files, m)?)?;
    m.add_function(wrap_pyfunction!(tlv_from_gz_file, m)?)?;
    Ok(())
}