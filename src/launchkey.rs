use midir::{MidiInput, MidiInputPort, MidiInputConnection, MidiOutput, MidiOutputPort, MidiOutputConnection};
use std::sync::{Arc, Mutex};

pub struct RGColor(u8);

impl RGColor {
    pub fn new(red: u8, green: u8) -> RGColor {
        RGColor(green.clamp(0, 3) * 16 + red.clamp(0, 3))
    }
}

impl Into<u8> for RGColor {
    fn into(self) -> u8 {
        let RGColor(name) = self;
        name
    }
}

impl From<u8> for RGColor {
    fn from(val: u8) -> RGColor {
        RGColor(val)
    }
}

#[derive(Debug)]
pub struct Message {
    pub timestamp: Option<u64>,
    pub status: u8,
    pub note: u8,
    pub data: u8,
}

impl Message {
    pub fn new(status: u8, note: u8, data: u8) -> Self { Message { timestamp: None, status, note, data } }
    pub fn with_timestamp(self, ts: u64) -> Self { Message { timestamp: Some(ts), ..self } }
    pub fn to_byte_vec(self) -> Vec<u8> {
        vec![self.status, self.note, self.data]
    }
}

pub struct PortPair (MidiInput, MidiOutput);
pub struct ConnectedPortPair (MidiInputConnection<()>, MidiOutputConnection);

impl PortPair {
    pub fn midi() -> Result<PortPair, failure::Error> {
        Ok(PortPair(
            MidiInput::new("Midi Input")?,
            MidiOutput::new("Midi Input")?
        ))
    }
    pub fn connect(self, input_port: &str, output_port: &str, event_list: Arc<Mutex<Vec<Message>>>) -> Result<ConnectedPortPair, failure::Error> {
        let input_ports = MidiInput::new("port finder")?.ports();
        let output_ports = MidiOutput::new("port finder")?.ports();
        let PortPair(input, output) = self;
        Ok(ConnectedPortPair(
            input.connect(
                &get_input_port_by_name(input_port)?.expect(format!("Could not find input port '{}'", input_port).as_str()),
                "Midi Input Port",
                move |ts, bytes, _| {
                    event_list
                        .lock()
                        .unwrap()
                        .push(Message::new(
                                *bytes.get(0).or(Some(&0)).unwrap(),
                                *bytes.get(1).or(Some(&0)).unwrap(),
                                *bytes.get(2).or(Some(&0)).unwrap()).with_timestamp(ts))
                }, ())?,
                output.connect(
                    &get_output_port_by_name(output_port)?.expect(format!("Could not find output port '{}'", output_port).as_str()),
                    "Midi Output Port")?
        ))
    }
}

impl ConnectedPortPair {
    pub fn send(&mut self, message: Message) {
        let ConnectedPortPair(input, output) = self;
        output.send(message.to_byte_vec().as_ref());
    }
    pub fn close(&mut self) {
        let ConnectedPortPair(input, output) = self;
        input.close();
        output.close();
    }
}

pub struct Launchkey {
    midi_port: ConnectedPortPair,
    incontrol_port: ConnectedPortPair,
    pub messages: Arc<Mutex<Vec<Message>>>
}

pub fn get_input_port_by_name(name: &str) -> Result<Option<MidiInputPort>, failure::Error> {
    let lister = MidiInput::new("Port Lister")?;
    // lister.ports().iter().for_each(|port| println!(" - input: {}", lister.port_name(port).expect("Could not read port name")));
    match lister.ports().iter().find(|port| lister.port_name(&port).unwrap_or("".to_string()).starts_with(name)) {
        Some(port) => Ok(Some(port.to_owned())),
        None => Ok(None)
    }
}

pub fn get_output_port_by_name(name: &str) -> Result<Option<MidiOutputPort>, failure::Error> {
    let lister = MidiOutput::new("Port Lister")?;
    // lister.ports().iter().for_each(|port| println!(" - output: {}", lister.port_name(port).expect("Could not read port name")));
    match lister.ports().iter().find(|port| lister.port_name(&port).unwrap_or("".to_string()).starts_with(name)) {
        Some(port) => Ok(Some(port.to_owned())),
        None => Ok(None)
    }
}

impl Launchkey {
    pub fn new() -> Result<Launchkey, failure::Error> {
        let messages = Arc::new(Mutex::new(Vec::<Message>::new()));
        Ok(Launchkey {
            midi_port: PortPair::midi()?.connect("Launchkey Mini", "Launchkey Mini", messages.clone())?,
            incontrol_port: PortPair::midi()?.connect("MIDIIN2", "MIDIOUT2", messages.clone())?,
            messages
        })
    }

    pub fn init(&mut self) {
        // Send InControl signal
        self.incontrol_port.send(Message::new(0x90, 0x0c, 0x7f))
    }

    pub fn set_pad(&mut self, pad: u8, color: RGColor) -> Result<(), failure::Error> {
        Ok(self.incontrol_port.send(Message::new(0x90, match pad {
            0..=7 => 96 + pad,
            8..=16 => 112 + pad - 8,
            _ => 0
        }, color.into())))
    }
    
    pub fn close(&mut self) {
        self.midi_port.close();
        self.incontrol_port.close();
    }
}

#[derive(Copy, Clone)]
pub enum LEDColor {
    Red,
    Green,
    Amber
}

impl Into<RGColor> for LEDColor {
    fn into(self) -> RGColor {
        match self {
            LEDColor::Red => RGColor::new(3, 0),
            LEDColor::Green => RGColor::new(0, 3),
            LEDColor::Amber => RGColor::new(3, 3)
        }
    }
}

pub struct LEDPad {
    pub pos: u8,
    pub on: bool,
    pub color: LEDColor,
}

impl LEDPad {
    pub fn new(pos: u8) -> Self {
        LEDPad { pos, on: false, color: LEDColor::Red }
    }

    pub fn red(&mut self) {
        self.color = LEDColor::Red;
    }

    pub fn green(&mut self) {
        self.color = LEDColor::Green;
    }

    pub fn amber(&mut self) {
        self.color = LEDColor::Amber;
    }

    pub fn toggle(&mut self) {
        self.on = !self.on;
    }

    pub fn on(&mut self) {
        self.on = true;
    }

    pub fn off(&mut self) {
        self.on = false;
    }

    pub fn show(&self, launchkey: &mut Launchkey) -> Result<(), failure::Error>  {
        launchkey.set_pad(self.pos, match self.on {
            true => self.color.into(),
            false => RGColor::new(0, 0)
        })
    }
}

pub struct LEDPadSet(Vec<LEDPad>);

impl LEDPadSet {
    pub fn new() -> LEDPadSet {
        let mut pad_vec = Vec::<LEDPad>::new();
        for i in 0..16 {
            pad_vec.push(LEDPad::new(i));
        }
        LEDPadSet(pad_vec)
    }

    pub fn pad(&mut self, pad: usize) -> &mut LEDPad {
        let LEDPadSet(pad_vec) = self;
        pad_vec.get_mut(pad).expect(format!("Could not get pad at pos {}", pad).as_str())
    }

    pub fn show(&self, launchkey: &mut Launchkey) -> Result<(), failure::Error> {
        let LEDPadSet(pad_vec) = self;
        for pad in pad_vec {
            pad.show(launchkey)?;
        }
        Ok(())
    }
}

