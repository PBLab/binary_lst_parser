extern crate failure;
extern crate bitreader;
extern crate filebuffer;

use std::fs;
use std::collections::HashMap;

use failure::Error;
use filebuffer::FileBuffer;
use bitreader::BitReader;

#[derive(Clone, Debug)]
pub struct DataLine {
    lost: u8,
    tag: u16,
    edge: bool,
    sweep: u16,
    time: u64,
}

impl DataLine {
    fn new(lost: u8, tag: u16, edge: bool, sweep: u16, time: u64) -> DataLine {
        DataLine { lost, tag, edge, sweep, time }
    }
}

/// Generate a HashMap where the keys are the channel numbers, and the values
/// are vectors containing data
fn create_channel_map(data_size: usize, active_channels: [u8; 6]) -> HashMap<u8, Vec<DataLine>> {
    let mut channel_map = HashMap::new();

    let vec = Vec::with_capacity(data_size + 1);
//            vec.set_len(data_size + 1);
    for (idx, is_active) in active_channels.iter().enumerate() {
        if is_active == &1u8 {
            channel_map.insert((idx + 1) as u8, vec.clone());
        }
    }
    channel_map
}

enum Timepatch {
    Tp0(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
        -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tp5(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
        -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tp1(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
        -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tp1a(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
         -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tp2a(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
         -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tp22(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
         -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tp32(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
         -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tp2(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
        -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tp5b(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
         -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    TpDb(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
         -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tpf3(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
         -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tp43(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
         -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tpc3(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
         -> Result<HashMap<u8, Vec<DataLine>>, Error>),
    Tp3(fn(&[u8], u64, &[u8; 4], HashMap<u8, Vec<DataLine>>)
        -> Result<HashMap<u8, Vec<DataLine>>, Error>),
}

impl Timepatch {
    fn new(tp: &str) -> Timepatch {
        match tp {
            "0" => Timepatch::Tp0(parse_no_sweep),
            "5" => Timepatch::Tp5(parse_with_sweep),
            "1" => Timepatch::Tp1(parse_no_sweep),
            "1a" => Timepatch::Tp1a(parse_with_sweep),
            "2a" => Timepatch::Tp2a(parse_with_sweep),
            "22" => Timepatch::Tp22(parse_no_sweep),
            "32" => Timepatch::Tp32(parse_with_sweep),
            "2" => Timepatch::Tp2(parse_no_sweep),
            "5b" => Timepatch::Tp5b(parse_with_sweep),
            "Db" => Timepatch::TpDb(parse_with_sweep),
            "f3" => Timepatch::Tpf3(parse_f3),
            "43" => Timepatch::Tp43(parse_no_sweep),
            "c3" => Timepatch::Tpc3(parse_no_sweep),
            "3" => Timepatch::Tp3(parse_no_sweep),
            _ => panic!("Invalid timepatch value."),
        }
    }
}


/// Parse data in file if timepatch == "f3"
fn parse_f3(data: &[u8], range: u64, bit_order: &[u8; 4],
            mut map_of_data: HashMap<u8, Vec<DataLine>>) -> Result<HashMap<u8, Vec<DataLine>>, Error> {
    let mut lost: u8;
    let mut tag: u16;
    let mut sweep: u16;
    let mut time: u64;
    let mut edge: bool;
    let mut chan: u8;

    let mut chunk_size: u8 = bit_order.iter().sum();
    chunk_size = (chunk_size  + 4u8) / 8u8;
    let mut reversed_vec = Vec::with_capacity(chunk_size as usize + 1);
    for cur_data in data.chunks(chunk_size as usize) {
        reversed_vec.truncate(0);
        reversed_vec.extend(cur_data.iter().rev());
        let mut reader = BitReader::new(&reversed_vec);
        tag = reader.read_u16(bit_order[1]).expect("(f3) tag read problem.");
        lost = reader.read_u8(bit_order[0]).expect("(f3) lost read problem.");
        sweep = reader.read_u16(bit_order[2]).expect("(f3) sweep read problem.");
        time = reader.read_u64(bit_order[3]).expect("(f3) time read problem");
        time = time + range * ((sweep - 1) as u64);
        edge = reader.read_bool().expect("(f3) edge read problem.");
        chan = reader.read_u8(3).expect("channel read problem.");


        // Populate a hashmap, each key being an input channel and the values are a vector
        // of DataLines
        map_of_data.get_mut(&chan).unwrap().push(DataLine::new(lost, tag, edge, sweep, time));
    }
    Ok(map_of_data)
}

/// Parse list files with a sweep counter
fn parse_with_sweep(data: &[u8], range: u64, bit_order: &[u8; 4],
                    mut map_of_data: HashMap<u8, Vec<DataLine>>) -> Result<HashMap<u8, Vec<DataLine>>, Error> {
    let mut lost: u8;
    let mut tag: u16;
    let mut sweep: u16;
    let mut time: u64;
    let mut edge: bool;
    let mut chan: u8;

    let mut chunk_size: u8 = bit_order.iter().sum();
    chunk_size = (chunk_size  + 4u8) / 8u8;
    let mut reversed_vec = Vec::with_capacity(chunk_size as usize + 1);
        for cur_data in data.chunks(chunk_size as usize) {
            reversed_vec.truncate(0);
            reversed_vec.extend(cur_data.iter().rev());
            let mut reader = BitReader::new(&reversed_vec);
            lost = reader.read_u8(bit_order[0]).expect("lost read problem.");
            tag = reader.read_u16(bit_order[1]).expect("tag read problem.");
            sweep = reader.read_u16(bit_order[2]).expect("sweep read problem.");
            time = reader.read_u64(bit_order[3]).expect("time read problem.");
            edge = reader.read_bool().expect("edge read problem.");
            chan = reader.read_u8(3).expect("channel read problem.");

            time = time + (range * ((sweep - 1) as u64));
            // Populate a hashmap, each key being an input channel and the values are a vector
            // of DataLines
            map_of_data.get_mut(&chan).unwrap().push(DataLine::new(lost, tag, edge, sweep, time));
        }

    Ok(map_of_data)
}

/// Parse list files without a sweep counter
fn parse_no_sweep(data: &[u8], range: u64, bit_order: &[u8; 4],
                  mut map_of_data: HashMap<u8, Vec<DataLine>>) -> Result<HashMap<u8, Vec<DataLine>>, Error> {
    let mut lost: u8;
    let mut tag: u16;
    let mut sweep: u16;
    let mut time: u64;
    let mut edge: bool;
    let mut chan: u8;


    let mut chunk_size: u8 = bit_order.iter().sum();
    chunk_size = (chunk_size  + 4u8) / 8u8;
    let mut reversed_vec = Vec::with_capacity(chunk_size as usize + 1);
        for cur_data in data.chunks(chunk_size as usize) {
            reversed_vec.truncate(0);
            reversed_vec.extend(cur_data.iter().rev());
            let mut reader = BitReader::new(&reversed_vec);
            lost = reader.read_u8(bit_order[0]).expect("lost read problem.");
            tag = reader.read_u16(bit_order[1]).expect("tag read problem.");
            sweep = reader.read_u16(bit_order[2]).expect("sweep read problem.");
            time = reader.read_u64(bit_order[3]).expect("time read problem.");
            edge = reader.read_bool().expect("edge read problem.");
            chan = reader.read_u8(3).expect("channel read problem.");
            // Populate a hashmap, each key being an input channel and the values are a vector
            // of DataLines
            map_of_data.get_mut(&chan).unwrap().push(DataLine::new(lost, tag, edge, sweep, time));
        }

    Ok(map_of_data)
}


/// Parse binary list files generated by a multiscaler
pub fn parse_lst(fname: String, start_of_data: usize, range: u64,
                 timepatch: String, channel_map: [u8; 6]) -> Result<HashMap<u8, Vec<DataLine>>, Error> {
    let data_with_headers = FileBuffer::open(&fname)?;
    let data = &data_with_headers[start_of_data..];
    let data_size: usize = (fs::metadata(&fname)?.len() - start_of_data as u64) as usize;
    let chan_map = create_channel_map(data_size, channel_map);
    let tp_enum = Timepatch::new(timepatch.as_str());
    let processed_data = match tp_enum {
        Timepatch::Tp0(func) => func(data, range, &[0, 0, 0, 12], chan_map),
        Timepatch::Tp5(func) => func(data, range, &[0, 0, 8, 20], chan_map),
        Timepatch::Tp1(func) => func(data, range, &[0, 0, 0, 28], chan_map),
        Timepatch::Tp1a(func) => func(data, range, &[0, 0, 16, 28], chan_map),
        Timepatch::Tp2a(func) => func(data, range, &[0, 8, 8, 28], chan_map),
        Timepatch::Tp22(func) => func(data, range, &[0, 8, 0, 36], chan_map),
        Timepatch::Tp32(func) => func(data, range, &[1, 0, 7, 36], chan_map),
        Timepatch::Tp2(func) => func(data, range, &[0, 0, 0, 44], chan_map),
        Timepatch::Tp5b(func) => func(data, range, &[1, 15, 16, 28], chan_map),
        Timepatch::TpDb(func) => func(data, range, &[0, 0, 16, 28], chan_map),
        Timepatch::Tpf3(func) => func(data, range, &[1, 16, 7, 36], chan_map),
        Timepatch::Tp43(func) => func(data, range, &[1, 15, 0, 44], chan_map),
        Timepatch::Tpc3(func) => func(data, range, &[0, 16, 0, 44], chan_map),
        Timepatch::Tp3(func) => func(data, range, &[1, 5, 0, 54], chan_map),
    };
    processed_data
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
