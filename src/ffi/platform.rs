use embassy_rp::i2c::{Blocking, I2c};
use embassy_rp::peripherals::I2C1;
use embedded_hal::i2c::{I2c as _, Operation};

type Bus = I2c<'static, I2C1, Blocking>;

#[repr(C)]
pub struct VL53L7CX_Platform {
    pub address: u16,
    pub i2c: *mut core::ffi::c_void,
}

pub struct Vl53l7cxCtx<'a> {
    bus: &'a mut Bus,
    address: u8,
}

impl Vl53l7cxCtx<'_> {
    /// Constructs a safe Rust context from the raw FFI platform struct.
    ///
    /// # Safety
    /// - `p` must be a valid, aligned, non-null pointer to a live `VL53L7CX_Platform`.
    /// - `p.i2c` must point to a live `Bus` with no other outstanding mutable references.
    pub unsafe fn from_raw(p: *mut VL53L7CX_Platform) -> Self {
        let p_ref = unsafe { &mut *p };
        let bus = unsafe { &mut *p_ref.i2c.cast::<Bus>() };
        let address =
            u8::try_from(p_ref.address >> 1).expect("VL53L7CX I2C address exceeds 7-bit range");

        Self { bus, address }
    }

    pub fn write_byte(&mut self, reg: u16, value: u8) -> Result<(), ()> {
        let [reg_hi, reg_lo] = reg.to_be_bytes();
        self.bus
            .write(self.address, &[reg_hi, reg_lo, value])
            .map_err(|_| ())
    }

    pub fn read_byte(&mut self, reg: u16, out: &mut u8) -> Result<(), ()> {
        self.bus
            .write_read(self.address, &reg.to_be_bytes(), core::slice::from_mut(out))
            .map_err(|_| ())
    }

    pub fn write_multi(&mut self, reg: u16, data: &[u8]) -> Result<(), ()> {
        self.bus
            .transaction(
                self.address,
                &mut [Operation::Write(&reg.to_be_bytes()), Operation::Write(data)],
            )
            .map_err(|_| ())
    }

    pub fn read_multi(&mut self, reg: u16, out: &mut [u8]) -> Result<(), ()> {
        self.bus
            .write_read(self.address, &reg.to_be_bytes(), out)
            .map_err(|_| ())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VL53L7CX_WrByte(p: *mut VL53L7CX_Platform, reg: u16, value: u8) -> u8 {
    let mut ctx = unsafe { Vl53l7cxCtx::from_raw(p) };
    ctx.write_byte(reg, value).map_or(1, |()| 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VL53L7CX_RdByte(p: *mut VL53L7CX_Platform, reg: u16, out: *mut u8) -> u8 {
    let mut ctx = unsafe { Vl53l7cxCtx::from_raw(p) };
    let out_ref = unsafe { &mut *out };

    ctx.read_byte(reg, out_ref).map_or(1, |()| 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VL53L7CX_WrMulti(
    p: *mut VL53L7CX_Platform,
    reg: u16,
    data: *const u8,
    size: u32,
) -> u8 {
    let mut ctx = unsafe { Vl53l7cxCtx::from_raw(p) };
    let payload = unsafe { core::slice::from_raw_parts(data, size as usize) };

    ctx.write_multi(reg, payload).map_or(1, |()| 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VL53L7CX_RdMulti(
    p: *mut VL53L7CX_Platform,
    reg: u16,
    out: *mut u8,
    size: u32,
) -> u8 {
    let mut ctx = unsafe { Vl53l7cxCtx::from_raw(p) };
    let buf = unsafe { core::slice::from_raw_parts_mut(out, size as usize) };

    ctx.read_multi(reg, buf).map_or(1, |()| 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VL53L7CX_WaitMs(_p: *mut VL53L7CX_Platform, ms: u32) -> u8 {
    embassy_time::block_for(embassy_time::Duration::from_millis(u64::from(ms)));
    0
}

/// No XSHUT pin is wired on this platform; non-zero tells the driver to fall
#[unsafe(no_mangle)]
pub const unsafe extern "C" fn VL53L7CX_Reset_Sensor(_p: *mut VL53L7CX_Platform) -> u8 {
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VL53L7CX_SwapBuffer(buffer: *mut u8, size: u16) {
    let slice = unsafe { core::slice::from_raw_parts_mut(buffer, size as usize) };

    for chunk in slice.chunks_exact_mut(4) {
        let val = u32::from_ne_bytes(chunk.try_into().unwrap());
        chunk.copy_from_slice(&val.swap_bytes().to_ne_bytes());
    }
}
