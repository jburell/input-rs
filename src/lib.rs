extern crate evdev;
use evdev::Device;
use std::sync::mpsc::Sender;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct TouchEvent {
    pos_x: f32,
    pos_y: f32,
}

pub struct InputHandler<C> {
    device: Arc<Mutex<Device>>,
    touch_handler: Option<C>,
    is_polling_events: std::sync::atomic::AtomicBool,
    polling_thread: Option<std::thread::JoinHandle<()>>,
}

impl<C> InputHandler<C> where C: FnMut(TouchEvent) {
    pub fn new(vendor: u16, product: u16) -> Result<Self, String> {
        let mut devices_matching = evdev::enumerate()
            .into_iter()
            .filter(|d| 
                d.input_id().vendor == vendor 
                && d.input_id().product == product)
            .collect::<Vec<Device>>();

        let device = if devices_matching.len() > 0 {
            Ok(devices_matching.remove(0))
        } else {
            Err(format!("Could not open device with: vendor:{}, product: {}", vendor, product))
        }?;

        Ok(InputHandler {
            device: Arc::new(Mutex::new(device)),
            touch_handler: None,
            is_polling_events: std::sync::atomic::AtomicBool::new(false),
            polling_thread: None,
        })
    }

    pub fn register_touch_handler(&mut self, event_handler: C) {
        self.touch_handler = Some(event_handler);
    }

    pub fn is_polling_events(&self) -> bool {
        self.is_polling_events.load(Ordering::SeqCst)
    }

    fn start_polling_thread(&mut self) {
        let dev = self.device.clone();
        self.polling_thread = Some(std::thread::spawn(move || {
            loop {
                let mut d = dev.lock().expect("Could not lock the device for polling");
                if let Ok(raw_events) = d.events_no_sync() {
                    raw_events.into_iter()
                        .fold(TouchEvent{
                            pos_x: 0f32,
                            pos_y: 0f32,
                        }, |acc, ev| {
                            println!("Event: {:?}", ev);
                            acc
                        });
                }; 
            }  
        }));
    }

    pub fn start_polling_events(&mut self) {
        let is_polling = self.is_polling_events.load(Ordering::Acquire);
        if !is_polling {
            self.start_polling_thread();
            self.is_polling_events.store(is_polling, Ordering::Release);
        } else {
            self.is_polling_events.store(is_polling, Ordering::Release);
        }
        /*chan.send(TouchEvent{
            pos_x = 0,
            pos_y = 0,
        })*/
    }
}