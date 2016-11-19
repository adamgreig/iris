use std::os::raw::c_char;
use std::ffi::CString;
use std::thread;
use std::sync::mpsc;

enum CSTVoice {}

#[link(name="flite_cmu_us_slt")]
extern {
    fn register_cmu_us_slt(voxdir: *const c_char) -> *mut CSTVoice;
}

#[link(name="flite_usenglish")]
extern {}

#[link(name="flite_cmulex")]
extern {}

#[link(name="flite")]
extern {
    fn flite_init();
    fn flite_text_to_speech(text: *const c_char, voice: *mut CSTVoice,
                            outtype: *const c_char) -> f32;
}

#[link(name="asound")]
extern {}

pub struct TTS {
    tx: mpsc::SyncSender<String>,
}

impl TTS {
    pub fn new() -> TTS {
        let (tx, rx) = mpsc::sync_channel::<String>(0);

        thread::spawn(move || {
            let voxdir = CString::new("").unwrap();
            let play = CString::new("play").unwrap();
            let v: *mut CSTVoice;

            unsafe {
                flite_init();
                v = register_cmu_us_slt(voxdir.as_ptr());
            }

            loop {
                for message in rx.recv() {
                    let t = CString::new(message).unwrap();
                    unsafe {
                        flite_text_to_speech(t.as_ptr(), v, play.as_ptr());
                    }
                }
            }
        });

        TTS { tx: tx }
    }

    pub fn say(&self, text: &str) {
        let _ = self.tx.send(text.to_string());
    }
}

pub fn init() {
}

pub fn say(text: &str) {
    let t = CString::new(text).unwrap();
    let voxdir = CString::new("").unwrap();
    let play = CString::new("play").unwrap();
    unsafe {
        let v: *mut CSTVoice = register_cmu_us_slt(voxdir.as_ptr());
        flite_text_to_speech(t.as_ptr(), v, play.as_ptr());
    }
}
