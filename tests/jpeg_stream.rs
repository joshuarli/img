use img::jpeg_stream::{ColorSpace, Decompress, ReadHeader, Source, SourceState};
use sha2::{Digest, Sha256};

const NULL: usize = usize::MAX;

struct Restartable {
    data: Vec<u8>,
    available: usize,
    next_read: usize,
    restart: usize,
    needs_restart: bool,
    last_set: usize,
}

impl Restartable {
    fn new(data: &[u8], available: usize) -> Self {
        Restartable { data: data.to_vec(), available, next_read: 0, restart: 0, needs_restart: false, last_set: NULL }
    }
}

impl Source for Restartable {
    fn data(&self) -> &[u8] {
        &self.data
    }

    fn fill_input_buffer(&mut self, state: &mut SourceState) -> bool {
        if self.needs_restart {
            self.needs_restart = false;
            self.next_read = self.restart;
        } else if self.last_set != state.next_input_byte {
            self.restart = self.next_read - state.bytes_in_buffer;
        }
        if self.next_read >= self.available {
            self.needs_restart = true;
            state.bytes_in_buffer = 0;
            state.next_input_byte = NULL;
            self.last_set = NULL;
            return false;
        }
        state.next_input_byte = self.next_read;
        state.bytes_in_buffer = self.available - self.next_read;
        self.last_set = self.next_read;
        self.next_read = self.available;
        true
    }

    fn skip_input_data(&mut self, state: &mut SourceState, num_bytes: i64) {
        let skip = num_bytes as usize;
        if skip < state.bytes_in_buffer {
            state.bytes_in_buffer -= skip;
            state.next_input_byte += skip;
        } else {
            self.next_read = self.next_read + skip - state.bytes_in_buffer;
            state.bytes_in_buffer = 0;
            state.next_input_byte = NULL;
        }
        self.restart = self.next_read - state.bytes_in_buffer;
        self.last_set = state.next_input_byte;
    }

    fn term_source(&mut self) {}
}

fn decode(data: &[u8], step: usize) -> Vec<u8> {
    let mut available = step.min(data.len());
    let mut decompress = Decompress::new(Restartable::new(data, available));
    let mut grow = |decompress: &mut Decompress<Restartable>| {
        available = (available + step).min(data.len());
        decompress.src.available = available;
    };
    while decompress.read_header(true).unwrap() == ReadHeader::Suspended {
        grow(&mut decompress);
    }
    decompress.out_color_space = ColorSpace::ExtBgra;
    while !decompress.start_decompress().unwrap() {
        grow(&mut decompress);
    }
    let stride = decompress.output_width as usize * 4;
    let mut pixels = vec![0u8; stride * decompress.output_height as usize];
    while decompress.output_scanline < decompress.output_height {
        let row = decompress.output_scanline as usize;
        if decompress.read_scanlines(&mut pixels[row * stride..(row + 1) * stride]).unwrap() == 0 {
            grow(&mut decompress);
        }
    }
    while !decompress.finish_decompress().unwrap() {
        grow(&mut decompress);
    }
    pixels
}

fn digest(pixels: &[u8]) -> String {
    Sha256::digest(pixels).iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn decodes_to_chromium_libjpeg_turbo_pixels() {
    let baseline = std::fs::read("tests/files/jpeg/baseline-420.jpg").unwrap();
    assert_eq!(digest(&decode(&baseline, usize::MAX)), "2efda59198d4fe11f3512bb74437b5dd0446d308baedaf0140070ba1dd128837");
    let progressive = std::fs::read("tests/files/jpeg/progressive-422.jpg").unwrap();
    assert_eq!(digest(&decode(&progressive, usize::MAX)), "96a2e0e9d925f7dd0102abeddcc26e85565cf9a18c6cc58c01d80511cfdb57dd");
}

#[test]
fn suspended_input_resumes_to_the_same_pixels() {
    for name in ["baseline-420.jpg", "progressive-422.jpg"] {
        let data = std::fs::read(format!("tests/files/jpeg/{name}")).unwrap();
        let whole = decode(&data, usize::MAX);
        for step in [1, 7, 64] {
            assert_eq!(decode(&data, step), whole, "{name} fed {step} bytes at a time");
        }
    }
}
